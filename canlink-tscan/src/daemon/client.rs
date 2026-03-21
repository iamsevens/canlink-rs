use super::codec::{read_frame, write_frame};
use super::protocol::{
    is_idempotent_op, ConnectResult, ErrorCode, HelloAck, Op, Request, Response, Status,
};
use crate::config::TscanDaemonConfig;
use canlink_hal::{CanError, CanResult};
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const PROTOCOL_VERSION: u32 = 1;
const DISCONNECT_RECOVERY_RETRY_DELAY_MS: u64 = 100;
type SpawnDaemonParts = (
    Child,
    ChildStdin,
    Receiver<io::Result<Response>>,
    JoinHandle<()>,
);

#[derive(Debug, Clone, Copy)]
pub struct InitParams {
    pub enable_fifo: bool,
    pub enable_error_frame: bool,
    pub use_hw_time: bool,
}

impl Default for InitParams {
    fn default() -> Self {
        Self {
            enable_fifo: true,
            enable_error_frame: false,
            use_hw_time: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CanBaudrateConfig {
    pub rate_kbps: f64,
    pub term: u8,
}

#[derive(Debug, Clone)]
pub struct CanFdBaudrateConfig {
    pub arb_kbps: f64,
    pub data_kbps: f64,
    pub ctrl_type: u8,
    pub ctrl_mode: u8,
    pub term: u8,
}

#[derive(Debug, Clone, Default)]
pub struct BackendStateCache {
    pub init: InitParams,
    pub serial: Option<String>,
    pub handle: Option<u64>,
    pub channel_count: Option<u8>,
    pub supports_canfd: Option<bool>,
    pub can_baud: HashMap<u8, CanBaudrateConfig>,
    pub canfd_baud: HashMap<u8, CanFdBaudrateConfig>,
    pub opened_channels: HashSet<u8>,
}

pub struct DaemonClient {
    config: TscanDaemonConfig,
    next_id: u64,
    cache: BackendStateCache,
    child: Child,
    stdin: ChildStdin,
    response_rx: Receiver<io::Result<Response>>,
    reader_handle: Option<JoinHandle<()>>,
}

impl DaemonClient {
    pub fn connect(config: &TscanDaemonConfig, init: InitParams) -> CanResult<Self> {
        let (child, stdin, response_rx, reader_handle) = spawn_daemon(config)?;
        let mut client = Self {
            config: config.clone(),
            next_id: 1,
            cache: BackendStateCache {
                init,
                ..BackendStateCache::default()
            },
            child,
            stdin,
            response_rx,
            reader_handle: Some(reader_handle),
        };
        client.bootstrap()?;
        Ok(client)
    }

    pub fn request_auto(&mut self, op: Op) -> CanResult<Response> {
        let timeout = self.timeout_for_op(&op);
        self.request(op, timeout)
    }

    pub fn request(&mut self, op: Op, timeout: Duration) -> CanResult<Response> {
        let request = self.next_request(op.clone());
        if is_disconnect_op(&op) && force_restart_on_disconnect() {
            return self.handle_disconnect_timeout(request.id, timeout);
        }
        match self.send_request_once(&request, timeout) {
            Ok(response) => {
                let response = ensure_response_ok(response)?;
                self.update_cache_for_success(&op, &response)?;
                Ok(response)
            }
            Err(failure) => self.handle_failure(request, op, failure, timeout),
        }
    }

    pub fn cache(&self) -> &BackendStateCache {
        &self.cache
    }

    pub fn cache_mut(&mut self) -> &mut BackendStateCache {
        &mut self.cache
    }

    pub fn clear_cache(&mut self) {
        self.cache.serial = None;
        self.cache.handle = None;
        self.cache.channel_count = None;
        self.cache.supports_canfd = None;
        self.cache.can_baud.clear();
        self.cache.canfd_baud.clear();
        self.cache.opened_channels.clear();
    }

    pub fn shutdown(&mut self) {
        self.kill_child();
    }

    fn timeout_for_op(&self, op: &Op) -> Duration {
        match op {
            Op::DisconnectByHandle { .. } | Op::DisconnectAll => {
                Duration::from_millis(self.config.disconnect_timeout_ms)
            }
            _ => Duration::from_millis(self.config.request_timeout_ms),
        }
    }

    fn bootstrap(&mut self) -> CanResult<()> {
        let hello = self.call_expect_ok(
            Op::Hello {
                protocol_version: PROTOCOL_VERSION,
                client_version: env!("CARGO_PKG_VERSION").to_string(),
            },
            Duration::from_millis(self.config.request_timeout_ms),
        )?;
        let ack: HelloAck = hello.decode_data()?;
        if ack.protocol_version != PROTOCOL_VERSION {
            return Err(CanError::InitializationFailed {
                reason: format!(
                    "protocol mismatch: daemon={}, client={}",
                    ack.protocol_version, PROTOCOL_VERSION
                ),
            });
        }

        self.call_expect_ok(
            Op::InitLib {
                enable_fifo: self.cache.init.enable_fifo,
                enable_error_frame: self.cache.init.enable_error_frame,
                use_hw_time: self.cache.init.use_hw_time,
            },
            Duration::from_millis(self.config.request_timeout_ms),
        )?;
        Ok(())
    }

    fn handle_failure(
        &mut self,
        request: Request,
        op: Op,
        failure: RequestFailure,
        timeout: Duration,
    ) -> CanResult<Response> {
        if matches!(request.op, Op::Finalize) {
            self.kill_child();
            return Err(failure.to_can_error(timeout, "finalize"));
        }

        if is_disconnect_op(&op) && (failure.is_timeout() || failure.is_io()) {
            return self.handle_disconnect_timeout(request.id, timeout);
        }

        self.restart_and_recover()?;
        if !is_retryable_after_recover(&op, &self.cache) {
            return Err(CanError::InitializationFailed {
                reason: format!(
                    "daemon recovered after {}, but '{}' is non-idempotent",
                    failure.describe(),
                    op_name(&op)
                ),
            });
        }

        let retry = self
            .send_request_once(&request, timeout)
            .map_err(|err| err.to_can_error(timeout, "retry"))?;
        let retry = ensure_response_ok(retry)?;
        self.update_cache_for_success(&op, &retry)?;
        Ok(retry)
    }

    fn handle_disconnect_timeout(
        &mut self,
        request_id: u64,
        timeout: Duration,
    ) -> CanResult<Response> {
        let retries = self.config.restart_max_retries;
        let mut last_err = String::new();
        for attempt in 0..=retries {
            self.kill_child();
            match self.try_disconnect_recovery(request_id, timeout) {
                Ok(response) => return Ok(response),
                Err(err) => {
                    last_err = format!("attempt {} failed: {err}", attempt + 1);
                    if attempt < retries {
                        thread::sleep(Duration::from_millis(DISCONNECT_RECOVERY_RETRY_DELAY_MS));
                    }
                }
            }
        }
        self.kill_child();
        Err(CanError::InitializationFailed {
            reason: format!(
                "disconnect timeout recovery failed after {} attempts: {last_err}",
                retries + 1
            ),
        })
    }

    fn restart_and_recover(&mut self) -> CanResult<()> {
        let retries = self.config.restart_max_retries;
        let mut last_err = String::new();
        for attempt in 0..=retries {
            self.kill_child();
            let result = self
                .respawn_and_bootstrap()
                .and_then(|_| self.recover_state());
            match result {
                Ok(()) => return Ok(()),
                Err(err) => {
                    last_err = format!("attempt {} failed: {err}", attempt + 1);
                }
            }
        }
        Err(CanError::InitializationFailed {
            reason: format!(
                "daemon restart failed after {} attempts: {last_err}",
                retries + 1
            ),
        })
    }

    fn respawn_and_bootstrap(&mut self) -> CanResult<()> {
        let (child, stdin, response_rx, reader_handle) = spawn_daemon(&self.config)?;
        self.child = child;
        self.stdin = stdin;
        self.response_rx = response_rx;
        self.reader_handle = Some(reader_handle);
        self.bootstrap()
    }

    fn recover_state(&mut self) -> CanResult<()> {
        let snapshot = self.cache.clone();
        if snapshot.handle.is_none() && snapshot.serial.is_none() {
            return Ok(());
        }

        let connect = self.call_expect_ok(
            Op::Connect {
                serial: snapshot.serial.unwrap_or_default(),
            },
            Duration::from_millis(self.config.request_timeout_ms),
        )?;
        let connected: ConnectResult = connect.decode_data()?;
        self.cache.handle = Some(connected.handle);
        self.cache.channel_count = Some(connected.channel_count);
        self.cache.supports_canfd = Some(connected.supports_canfd);
        self.cache.serial = Some(connected.serial);

        let mut can_baud_channels = snapshot.can_baud.keys().copied().collect::<Vec<_>>();
        can_baud_channels.sort_unstable();
        for channel in can_baud_channels {
            if let Some(cfg) = snapshot.can_baud.get(&channel) {
                let handle = self.cache.handle.unwrap_or_default();
                self.call_expect_ok(
                    Op::ConfigCanBaudrate {
                        handle,
                        channel,
                        rate_kbps: cfg.rate_kbps,
                        term: cfg.term,
                    },
                    Duration::from_millis(self.config.request_timeout_ms),
                )?;
                self.cache.can_baud.insert(channel, cfg.clone());
            }
        }

        let mut canfd_baud_channels = snapshot.canfd_baud.keys().copied().collect::<Vec<_>>();
        canfd_baud_channels.sort_unstable();
        for channel in canfd_baud_channels {
            if let Some(cfg) = snapshot.canfd_baud.get(&channel) {
                let handle = self.cache.handle.unwrap_or_default();
                self.call_expect_ok(
                    Op::ConfigCanfdBaudrate {
                        handle,
                        channel,
                        arb_kbps: cfg.arb_kbps,
                        data_kbps: cfg.data_kbps,
                        ctrl_type: cfg.ctrl_type,
                        ctrl_mode: cfg.ctrl_mode,
                        term: cfg.term,
                    },
                    Duration::from_millis(self.config.request_timeout_ms),
                )?;
                self.cache.canfd_baud.insert(channel, cfg.clone());
            }
        }

        let mut opened_channels = snapshot.opened_channels.iter().copied().collect::<Vec<_>>();
        opened_channels.sort_unstable();
        for channel in opened_channels {
            let handle = self.cache.handle.unwrap_or_default();
            self.call_expect_ok(
                Op::OpenChannel { handle, channel },
                Duration::from_millis(self.config.request_timeout_ms),
            )?;
            self.cache.opened_channels.insert(channel);
        }
        Ok(())
    }

    fn update_cache_for_success(&mut self, op: &Op, response: &Response) -> CanResult<()> {
        match op {
            Op::Connect { .. } => {
                let connected: ConnectResult = response.decode_data()?;
                self.cache.handle = Some(connected.handle);
                self.cache.channel_count = Some(connected.channel_count);
                self.cache.supports_canfd = Some(connected.supports_canfd);
                self.cache.serial = Some(connected.serial);
            }
            Op::DisconnectByHandle { .. } | Op::DisconnectAll | Op::Finalize => {
                self.clear_cache();
            }
            Op::OpenChannel { channel, .. } => {
                self.cache.opened_channels.insert(*channel);
            }
            Op::CloseChannel { channel, .. } => {
                self.cache.opened_channels.remove(channel);
            }
            Op::ConfigCanBaudrate {
                channel,
                rate_kbps,
                term,
                ..
            } => {
                self.cache.can_baud.insert(
                    *channel,
                    CanBaudrateConfig {
                        rate_kbps: *rate_kbps,
                        term: *term,
                    },
                );
            }
            Op::ConfigCanfdBaudrate {
                channel,
                arb_kbps,
                data_kbps,
                ctrl_type,
                ctrl_mode,
                term,
                ..
            } => {
                self.cache.canfd_baud.insert(
                    *channel,
                    CanFdBaudrateConfig {
                        arb_kbps: *arb_kbps,
                        data_kbps: *data_kbps,
                        ctrl_type: *ctrl_type,
                        ctrl_mode: *ctrl_mode,
                        term: *term,
                    },
                );
            }
            _ => {}
        }
        Ok(())
    }

    fn call_expect_ok(&mut self, op: Op, timeout: Duration) -> CanResult<Response> {
        let request = self.next_request(op);
        let response = self
            .send_request_once(&request, timeout)
            .map_err(|err| err.to_can_error(timeout, "request"))?;
        ensure_response_ok(response)
    }

    fn next_request(&mut self, op: Op) -> Request {
        let id = self.next_id;
        self.next_id += 1;
        Request::new(id, op)
    }

    fn send_request_once(
        &mut self,
        request: &Request,
        timeout: Duration,
    ) -> Result<Response, RequestFailure> {
        write_frame(&mut self.stdin, request)
            .map_err(|err| RequestFailure::io(format!("write request failed: {err}")))?;

        match self.response_rx.recv_timeout(timeout) {
            Ok(Ok(response)) => {
                if response.id != request.id {
                    return Err(RequestFailure::protocol(format!(
                        "response id mismatch: expected {}, got {}",
                        request.id, response.id
                    )));
                }
                Ok(response)
            }
            Ok(Err(err)) => Err(RequestFailure::io(format!("read response failed: {err}"))),
            Err(RecvTimeoutError::Timeout) => Err(RequestFailure::Timeout),
            Err(RecvTimeoutError::Disconnected) => {
                Err(RequestFailure::io("daemon pipe disconnected".to_string()))
            }
        }
    }

    fn kill_child(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(handle) = self.reader_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for DaemonClient {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn spawn_daemon(config: &TscanDaemonConfig) -> CanResult<SpawnDaemonParts> {
    let daemon_program = resolve_daemon_program(config);
    let mut child = Command::new(daemon_program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|err| CanError::InitializationFailed {
            reason: format!("failed to start daemon: {err}"),
        })?;

    let stdin = child.stdin.take().ok_or(CanError::InitializationFailed {
        reason: "failed to open daemon stdin".to_string(),
    })?;
    let stdout = child.stdout.take().ok_or(CanError::InitializationFailed {
        reason: "failed to open daemon stdout".to_string(),
    })?;
    let (response_rx, reader_handle) = spawn_reader_thread(stdout);
    Ok((child, stdin, response_rx, reader_handle))
}

fn spawn_reader_thread(stdout: ChildStdout) -> (Receiver<io::Result<Response>>, JoinHandle<()>) {
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let mut stdout = stdout;
        loop {
            let frame = read_frame::<_, Response>(&mut stdout);
            match frame {
                Ok(response) => {
                    if tx.send(Ok(response)).is_err() {
                        return;
                    }
                }
                Err(err) => {
                    let _ = tx.send(Err(err));
                    return;
                }
            }
        }
    });
    (rx, handle)
}

fn resolve_daemon_program(config: &TscanDaemonConfig) -> PathBuf {
    let current_exe = std::env::current_exe().ok();
    resolve_daemon_program_from(config.daemon_path.as_deref(), current_exe.as_deref())
}

fn resolve_daemon_program_from(daemon_path: Option<&str>, current_exe: Option<&Path>) -> PathBuf {
    if let Some(path) = daemon_path {
        return PathBuf::from(path);
    }

    if let Some(current_exe) = current_exe {
        for candidate in daemon_program_candidates(current_exe) {
            if candidate.exists() {
                return candidate;
            }
        }
    }

    PathBuf::from(daemon_executable_name())
}

fn daemon_program_candidates(current_exe: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(dir) = current_exe.parent() {
        candidates.push(dir.join(daemon_executable_name()));
        if let Some(parent) = dir.parent() {
            candidates.push(parent.join(daemon_executable_name()));
        }
    }
    candidates
}

fn daemon_executable_name() -> &'static str {
    #[cfg(windows)]
    {
        "canlink-tscan-daemon.exe"
    }
    #[cfg(not(windows))]
    {
        "canlink-tscan-daemon"
    }
}

fn ensure_response_ok(response: Response) -> CanResult<Response> {
    if response.is_ok() {
        return Ok(response);
    }
    Err(map_response_error(&response))
}

pub fn map_response_error(response: &Response) -> CanError {
    let detail = if response.message.is_empty() {
        format!("code={}", response.code)
    } else {
        format!("code={}, message={}", response.code, response.message)
    };
    match ErrorCode::from_u32(response.code) {
        Some(ErrorCode::LibTscanError) => CanError::Other {
            message: format!("libtscan error: {detail}"),
        },
        Some(ErrorCode::InvalidParams) | Some(ErrorCode::InvalidIndex) => {
            CanError::InvalidParameter {
                parameter: "daemon_request".to_string(),
                reason: detail,
            }
        }
        Some(ErrorCode::InvalidHandle) => CanError::InvalidState {
            expected: "valid handle".to_string(),
            current: detail,
        },
        Some(ErrorCode::InvalidChannel) => CanError::ChannelNotFound { channel: 0, max: 0 },
        Some(ErrorCode::NoDevice) => CanError::DeviceNotFound { device: detail },
        Some(ErrorCode::AlreadyConnected) | Some(ErrorCode::DeviceBusy) => {
            CanError::InitializationFailed { reason: detail }
        }
        Some(ErrorCode::ProtocolError) => CanError::InvalidFormat { reason: detail },
        _ => CanError::Other {
            message: format!("daemon error: {detail}"),
        },
    }
}

fn is_disconnect_op(op: &Op) -> bool {
    matches!(op, Op::DisconnectByHandle { .. } | Op::DisconnectAll)
}

fn is_retryable_after_recover(op: &Op, cache: &BackendStateCache) -> bool {
    if is_idempotent_op(op) {
        return true;
    }
    // First CONNECT before we hold a handle can be safely retried after daemon restart.
    matches!(op, Op::Connect { .. }) && cache.handle.is_none()
}

fn op_name(op: &Op) -> &'static str {
    match op {
        Op::Hello { .. } => "HELLO",
        Op::InitLib { .. } => "INIT_LIB",
        Op::Scan => "SCAN",
        Op::GetDeviceInfo { .. } => "GET_DEVICE_INFO",
        Op::Connect { .. } => "CONNECT",
        Op::DisconnectByHandle { .. } => "DISCONNECT_BY_HANDLE",
        Op::DisconnectAll => "DISCONNECT_ALL",
        Op::OpenChannel { .. } => "OPEN_CHANNEL",
        Op::CloseChannel { .. } => "CLOSE_CHANNEL",
        Op::ConfigCanBaudrate { .. } => "CONFIG_CAN_BAUDRATE",
        Op::ConfigCanfdBaudrate { .. } => "CONFIG_CANFD_BAUDRATE",
        Op::SendCan { .. } => "SEND_CAN",
        Op::SendCanfd { .. } => "SEND_CANFD",
        Op::RecvCan { .. } => "RECV_CAN",
        Op::RecvCanfd { .. } => "RECV_CANFD",
        Op::GetCapability { .. } => "GET_CAPABILITY",
        Op::Finalize => "FINALIZE",
    }
}

fn force_restart_on_disconnect() -> bool {
    match std::env::var("CANLINK_TSCAN_FORCE_RESTART_ON_DISCONNECT") {
        Ok(value) => {
            let value = value.trim().to_ascii_lowercase();
            !matches!(value.as_str(), "0" | "false" | "no" | "off")
        }
        Err(_) => false,
    }
}

#[derive(Debug)]
enum RequestFailure {
    Timeout,
    Io(String),
    Protocol(String),
}

impl RequestFailure {
    fn io(detail: String) -> Self {
        Self::Io(detail)
    }

    fn protocol(detail: String) -> Self {
        Self::Protocol(detail)
    }

    fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout)
    }

    fn is_io(&self) -> bool {
        matches!(self, Self::Io(_))
    }

    fn describe(&self) -> String {
        match self {
            Self::Timeout => "timeout".to_string(),
            Self::Io(detail) => format!("io error: {detail}"),
            Self::Protocol(detail) => format!("protocol error: {detail}"),
        }
    }

    fn to_can_error(&self, timeout: Duration, context: &str) -> CanError {
        match self {
            Self::Timeout => CanError::Timeout {
                timeout_ms: timeout.as_millis() as u64,
            },
            Self::Io(detail) | Self::Protocol(detail) => CanError::InitializationFailed {
                reason: format!("{context}: {detail}"),
            },
        }
    }
}

impl DaemonClient {
    fn try_disconnect_recovery(
        &mut self,
        request_id: u64,
        timeout: Duration,
    ) -> CanResult<Response> {
        self.respawn_and_bootstrap()?;
        self.call_expect_ok(Op::Scan, timeout)?;
        let serial = self.cache.serial.clone().unwrap_or_default();
        self.call_expect_ok(Op::Connect { serial }, timeout)?;

        self.kill_child();
        self.clear_cache();
        Ok(Response {
            id: request_id,
            status: Status::Ok,
            code: ErrorCode::Ok as u32,
            message: String::new(),
            data: serde_json::json!({}),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_daemon_program_from;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn daemon_resolution_uses_configured_path_first() {
        let path = resolve_daemon_program_from(Some("D:\\custom\\daemon.exe"), None);
        assert_eq!(path, PathBuf::from("D:\\custom\\daemon.exe"));
    }

    #[test]
    fn daemon_resolution_falls_back_to_parent_of_examples_dir() {
        let root = unique_temp_dir("daemon-resolution");
        let debug_dir = root.join("target").join("debug");
        let examples_dir = debug_dir.join("examples");
        fs::create_dir_all(&examples_dir).expect("create examples dir failed");
        let exe_path = examples_dir.join("disconnect_client_stress.exe");
        fs::write(&exe_path, b"exe").expect("write test exe failed");

        let daemon_path = debug_dir.join(daemon_name());
        fs::write(&daemon_path, b"daemon").expect("write daemon failed");

        let resolved = resolve_daemon_program_from(None, Some(exe_path.as_path()));

        assert_eq!(resolved, daemon_path);
        let _ = fs::remove_dir_all(root);
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time error")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("canlink-tscan-client-{prefix}-{nanos}"));
        fs::create_dir_all(&path).expect("create temp dir failed");
        path
    }

    fn daemon_name() -> &'static str {
        #[cfg(windows)]
        {
            "canlink-tscan-daemon.exe"
        }
        #[cfg(not(windows))]
        {
            "canlink-tscan-daemon"
        }
    }
}
