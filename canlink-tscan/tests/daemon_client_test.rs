use canlink_tscan::daemon::client::{DaemonClient, InitParams};
use canlink_tscan::daemon::Op;
use canlink_tscan::TscanDaemonConfig;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn unique_trace_file() -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time error")
        .as_nanos();
    std::env::temp_dir().join(format!("canlink-tscan-daemon-client-trace-{nanos}.log"))
}

fn stub_daemon_path() -> String {
    if let Some(path) = option_env!("CARGO_BIN_EXE_canlink-tscan-daemon-stub") {
        return path.to_string();
    }

    let fallback = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
        .and_then(|deps| deps.parent().map(|p| p.to_path_buf()))
        .map(|debug_dir| debug_dir.join("canlink-tscan-daemon-stub.exe"))
        .expect("failed to build fallback stub path");
    fallback.to_string_lossy().to_string()
}

fn base_config() -> TscanDaemonConfig {
    TscanDaemonConfig {
        use_daemon: true,
        daemon_path: Some(stub_daemon_path()),
        request_timeout_ms: 400,
        disconnect_timeout_ms: 300,
        restart_max_retries: 1,
        recv_timeout_ms: 0,
    }
}

#[test]
fn disconnect_timeout_triggers_restart() {
    let _guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|err| err.into_inner());

    let trace_path = unique_trace_file();
    std::env::set_var("HANG_ON_OP", "DISCONNECT_BY_HANDLE");
    std::env::set_var("TRACE_PATH", &trace_path);
    std::env::remove_var("PROTOCOL_VERSION");

    let mut client = DaemonClient::connect(&base_config(), InitParams::default())
        .expect("client connect failed");
    let connect = client
        .request_auto(Op::Connect {
            serial: String::new(),
        })
        .expect("connect op failed");
    assert!(connect.is_ok());

    let handle = client.cache().handle.expect("missing daemon handle");
    let response = client
        .request_auto(Op::DisconnectByHandle { handle })
        .expect("disconnect request failed");
    assert!(response.is_ok());

    client.shutdown();
    std::env::remove_var("HANG_ON_OP");
    std::env::remove_var("TRACE_PATH");

    let trace = std::fs::read_to_string(trace_path).expect("read trace file failed");
    let hello_count = trace.lines().filter(|line| *line == "HELLO").count();
    assert!(
        hello_count >= 2,
        "expected at least 2 HELLO entries, got {hello_count}"
    );
}

#[test]
fn disconnect_force_restart_skips_vendor_call() {
    let _guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|err| err.into_inner());

    let trace_path = unique_trace_file();
    std::env::set_var("TRACE_PATH", &trace_path);
    std::env::set_var("HANG_ON_OP", "DISCONNECT_BY_HANDLE");
    std::env::set_var("CANLINK_TSCAN_FORCE_RESTART_ON_DISCONNECT", "1");
    std::env::remove_var("PROTOCOL_VERSION");

    let mut client = DaemonClient::connect(&base_config(), InitParams::default())
        .expect("client connect failed");
    let connect = client
        .request_auto(Op::Connect {
            serial: String::new(),
        })
        .expect("connect op failed");
    assert!(connect.is_ok());

    let handle = client.cache().handle.expect("missing daemon handle");
    let response = client
        .request_auto(Op::DisconnectByHandle { handle })
        .expect("disconnect request failed");
    assert!(response.is_ok());

    client.shutdown();
    std::env::remove_var("HANG_ON_OP");
    std::env::remove_var("TRACE_PATH");
    std::env::remove_var("CANLINK_TSCAN_FORCE_RESTART_ON_DISCONNECT");

    let trace = std::fs::read_to_string(trace_path).expect("read trace file failed");
    assert!(
        !trace
            .lines()
            .any(|line| line.trim() == "OP:DISCONNECT_BY_HANDLE"),
        "expected disconnect to skip vendor call, trace was:\n{trace}"
    );
}

#[test]
fn disconnect_default_calls_vendor() {
    let _guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|err| err.into_inner());

    let trace_path = unique_trace_file();
    std::env::set_var("TRACE_PATH", &trace_path);
    std::env::set_var("HANG_ON_OP", "DISCONNECT_BY_HANDLE");
    std::env::remove_var("CANLINK_TSCAN_FORCE_RESTART_ON_DISCONNECT");
    std::env::remove_var("PROTOCOL_VERSION");

    let mut client = DaemonClient::connect(&base_config(), InitParams::default())
        .expect("client connect failed");
    let connect = client
        .request_auto(Op::Connect {
            serial: String::new(),
        })
        .expect("connect op failed");
    assert!(connect.is_ok());

    let handle = client.cache().handle.expect("missing daemon handle");
    let response = client
        .request_auto(Op::DisconnectByHandle { handle })
        .expect("disconnect request failed");
    assert!(response.is_ok());

    client.shutdown();
    std::env::remove_var("HANG_ON_OP");
    std::env::remove_var("TRACE_PATH");

    let trace = std::fs::read_to_string(trace_path).expect("read trace file failed");
    assert!(
        trace
            .lines()
            .any(|line| line.trim() == "OP:DISCONNECT_BY_HANDLE"),
        "expected disconnect to call vendor op, trace was:\n{trace}"
    );
}

#[test]
fn disconnect_exit_is_recovered() {
    let _guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|err| err.into_inner());

    std::env::set_var("EXIT_ON_OP_ONCE", "DISCONNECT_BY_HANDLE");
    std::env::remove_var("CANLINK_TSCAN_FORCE_RESTART_ON_DISCONNECT");
    std::env::remove_var("PROTOCOL_VERSION");

    let mut client = DaemonClient::connect(&base_config(), InitParams::default())
        .expect("client connect failed");
    let connect = client
        .request_auto(Op::Connect {
            serial: String::new(),
        })
        .expect("connect op failed");
    assert!(connect.is_ok());

    let handle = client.cache().handle.expect("missing daemon handle");
    let response = client
        .request_auto(Op::DisconnectByHandle { handle })
        .expect("disconnect request failed");
    assert!(response.is_ok());

    client.shutdown();
    std::env::remove_var("EXIT_ON_OP_ONCE");
}

#[test]
fn disconnect_timeout_recovery_retries_connect_validation() {
    let _guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|err| err.into_inner());

    let trace_path = unique_trace_file();
    std::env::set_var("TRACE_PATH", &trace_path);
    std::env::set_var("HANG_ON_OP", "DISCONNECT_BY_HANDLE");
    std::env::remove_var("EXIT_ON_OP_ONCE");
    std::env::remove_var("CANLINK_TSCAN_FORCE_RESTART_ON_DISCONNECT");
    std::env::remove_var("PROTOCOL_VERSION");

    let mut config = base_config();
    config.restart_max_retries = 1;

    let mut client =
        DaemonClient::connect(&config, InitParams::default()).expect("client connect failed");
    let connect = client
        .request_auto(Op::Connect {
            serial: String::new(),
        })
        .expect("connect op failed");
    assert!(connect.is_ok());

    std::env::set_var("EXIT_ON_OP_ONCE", "CONNECT");

    let handle = client.cache().handle.expect("missing daemon handle");
    let response = client
        .request_auto(Op::DisconnectByHandle { handle })
        .expect("disconnect request failed");
    assert!(response.is_ok());

    client.shutdown();
    std::env::remove_var("EXIT_ON_OP_ONCE");
    std::env::remove_var("HANG_ON_OP");
    std::env::remove_var("TRACE_PATH");

    let trace = std::fs::read_to_string(trace_path).expect("read trace file failed");
    let hello_count = trace.lines().filter(|line| *line == "HELLO").count();
    assert!(
        hello_count >= 3,
        "expected at least 3 HELLO entries after retry recovery, got {hello_count}"
    );
}

#[test]
fn hello_version_mismatch_returns_error() {
    let _guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|err| err.into_inner());

    std::env::remove_var("HANG_ON_OP");
    std::env::remove_var("TRACE_PATH");
    std::env::set_var("PROTOCOL_VERSION", "2");

    let result = DaemonClient::connect(&base_config(), InitParams::default());
    assert!(result.is_err(), "expected version mismatch to fail");

    std::env::remove_var("PROTOCOL_VERSION");
}

#[test]
fn first_connect_is_retryable_after_daemon_restart() {
    let _guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|err| err.into_inner());

    let trace_path = unique_trace_file();
    std::env::set_var("TRACE_PATH", &trace_path);
    std::env::set_var("EXIT_ON_OP_ONCE", "CONNECT");
    std::env::remove_var("HANG_ON_OP");
    std::env::remove_var("PROTOCOL_VERSION");

    let mut client = DaemonClient::connect(&base_config(), InitParams::default())
        .expect("client connect failed");
    let result = client.request_auto(Op::Connect {
        serial: String::new(),
    });

    std::env::remove_var("TRACE_PATH");
    std::env::remove_var("EXIT_ON_OP_ONCE");

    let response = result.expect("expected CONNECT retry after restart");
    assert!(response.is_ok());
    assert!(
        client.cache().handle.is_some(),
        "expected daemon handle cached after successful retry"
    );
}
