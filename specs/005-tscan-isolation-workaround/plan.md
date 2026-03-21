# 实施计划: TSCan 断开卡死隔离规避

> 说明：本文件为 `specs` 目录下的标准计划文档，使用复选框（`- [ ]`）跟踪执行步骤。

**Goal:** 在 `canlink-tscan` 中引入 daemon 隔离与 JSON IPC，默认规避 LibTSCAN 断开卡死，同时提供可配置开关与可测试的无硬件 stub。

**Architecture:** `TSCanBackend` 默认通过 `canlink-tscan-daemon` 子进程访问 LibTSCAN。主进程负责协议编解码、超时检测、重启与状态恢复；子进程仅做最小调用与返回。配置从 `BackendConfig` → `canlink-tscan.toml` → 默认值。

**Tech Stack:** Rust 2021, `std::process`, `serde`/`serde_json`, `toml`, `thiserror`（已有）

---

## 文件结构（先锁定职责）
- 新增：`canlink-tscan\src\config.rs`
  - 解析 `canlink-tscan.toml` 与 `BackendConfig.parameters`，产出 `TscanDaemonConfig`
- 新增：`canlink-tscan\src\daemon\mod.rs`
  - 汇总 IPC 结构、客户端、服务端共用类型
- 新增：`canlink-tscan\src\daemon\protocol.rs`
  - JSON 协议结构体、错误码、serde 映射
- 新增：`canlink-tscan\src\daemon\codec.rs`
  - 长度前缀 JSON 读写与最大帧校验
- 新增：`canlink-tscan\src\daemon\client.rs`
  - daemon 进程管理、请求发送、超时、重启、状态恢复
- 新增：`canlink-tscan\src\daemon\server.rs`
  - daemon 服务端逻辑（处理请求、调用 LibTSCAN）
- 新增：`canlink-tscan\src\bin\canlink-tscan-daemon.rs`
  - daemon 二进制入口
- 新增：`canlink-tscan\src\bin\canlink-tscan-daemon-stub.rs`
  - 测试用 stub（无硬件）
- 修改：`canlink-tscan\src\backend.rs`
  - 接入 daemon 与配置、状态缓存与恢复
- 修改：`canlink-tscan\src\lib.rs`
  - 导出配置/daemon 相关类型（如需要）
- 修改：`canlink-tscan\Cargo.toml`
  - 增加 `serde`, `serde_json`, `toml` 依赖与 bin 声明
- 新增测试：
  - `canlink-tscan\tests\daemon_protocol_test.rs`
  - `canlink-tscan\tests\daemon_client_test.rs`
  - `canlink-tscan\tests\config_test.rs`
- 文档更新：
  - `canlink-tscan\README.md`
  - `docs\user-guide.md`（追加配置说明）

---

## Chunk 1: 配置解析 + IPC 协议与编解码

### Task 1: 配置结构与优先级解析

**Files:**
- Create: `canlink-tscan\src\config.rs`
- Modify: `canlink-tscan\src\lib.rs`
- Test: `canlink-tscan\tests\config_test.rs`

- [ ] **Step 1: 写一个失败的配置解析测试**

```rust
#[test]
fn config_precedence_backend_over_file_over_default() {
    struct DirGuard(std::path::PathBuf);
    impl Drop for DirGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.0);
        }
    }

    let temp = std::env::temp_dir().join(format!("canlink-tscan-test-{}", std::process::id()));
    std::fs::create_dir_all(&temp).unwrap();
    std::fs::write(
        temp.join("canlink-tscan.toml"),
        r#"
use_daemon = true
request_timeout_ms = 2222
disconnect_timeout_ms = 3333
"#,
    )
    .unwrap();
    let prev = std::env::current_dir().unwrap();
    let _guard = DirGuard(prev);
    std::env::set_current_dir(&temp).unwrap();

    let mut cfg = BackendConfig::new("tscan");
    cfg.parameters
        .insert("use_daemon".into(), toml::Value::Boolean(false));
    cfg.parameters.insert(
        "request_timeout_ms".into(),
        toml::Value::Integer(1111),
    );

    let merged = TscanDaemonConfig::resolve(&cfg).unwrap();
    assert_eq!(merged.use_daemon, false); // BackendConfig 覆盖 file
    assert_eq!(merged.request_timeout_ms, 1111); // BackendConfig 覆盖 file
    assert_eq!(merged.disconnect_timeout_ms, 3333); // file 覆盖 default
    assert_eq!(merged.restart_max_retries, 3); // default
}
```

- [ ] **Step 2: 运行测试并确认失败**

Run: `cargo test -p canlink-tscan --test config_test`
Expected: FAIL（结构或解析函数不存在）

- [ ] **Step 3: 最小实现配置结构**

```rust
#[derive(Debug, Clone, Deserialize, Default)]
pub struct FileConfig {
    pub use_daemon: Option<bool>,
    pub daemon_path: Option<String>,
    pub request_timeout_ms: Option<u64>,
    pub disconnect_timeout_ms: Option<u64>,
    pub restart_max_retries: Option<u32>,
    pub recv_timeout_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct TscanDaemonConfig {
    pub use_daemon: bool,
    pub daemon_path: Option<String>,
    pub request_timeout_ms: u64,
    pub disconnect_timeout_ms: u64,
    pub restart_max_retries: u32,
    pub recv_timeout_ms: u64,
}
```

- [ ] **Step 4: 实现解析与优先级合并**

```rust
impl TscanDaemonConfig {
    pub fn defaults() -> Self { /* use_daemon=true, request=2000, disconnect=3000, restart=3, recv=0 */ }

    pub fn load_file(path: &Path) -> Result<Option<FileConfig>, CanError> {
        if (!path.exists()) { return Ok(None); }
        let text = std::fs::read_to_string(path)
            .map_err(|e| CanError::ConfigError { reason: e.to_string() })?;
        let cfg: FileConfig = toml::from_str(&text)
            .map_err(|e| CanError::ConfigError { reason: e.to_string() })?;
        Ok(Some(cfg))
    }

    pub fn apply_file(&mut self, file: FileConfig) {
        if let Some(v) = file.use_daemon { self.use_daemon = v; }
        if let Some(v) = file.daemon_path { self.daemon_path = Some(v); }
        if let Some(v) = file.request_timeout_ms { self.request_timeout_ms = v; }
        if let Some(v) = file.disconnect_timeout_ms { self.disconnect_timeout_ms = v; }
        if let Some(v) = file.restart_max_retries { self.restart_max_retries = v; }
        if let Some(v) = file.recv_timeout_ms { self.recv_timeout_ms = v; }
    }

    pub fn apply_backend(&mut self, cfg: &BackendConfig) {
        if let Some(v) = cfg.get_bool("use_daemon") { self.use_daemon = v; }
        if let Some(v) = cfg.get_string("daemon_path") { self.daemon_path = Some(v.to_string()); }
        if let Some(v) = cfg.get_int("request_timeout_ms") { self.request_timeout_ms = v.max(0) as u64; }
        if let Some(v) = cfg.get_int("disconnect_timeout_ms") { self.disconnect_timeout_ms = v.max(0) as u64; }
        if let Some(v) = cfg.get_int("restart_max_retries") { self.restart_max_retries = v.max(0) as u32; }
        if let Some(v) = cfg.get_int("recv_timeout_ms") { self.recv_timeout_ms = v.max(0) as u64; }
    }

    pub fn resolve(cfg: &BackendConfig) -> Result<Self, CanError> {
        let mut out = Self::defaults();
        if let Some(file) = Self::load_file(Path::new("canlink-tscan.toml"))? {
            out.apply_file(file);
        }
        out.apply_backend(cfg);
        Ok(out)
    }
}
```

- [ ] **Step 5: 运行测试并确认通过**

Run: `cargo test -p canlink-tscan --test config_test`
Expected: PASS

- [ ] **Step 6: 提交**

```bash
git add canlink-tscan\src\config.rs canlink-tscan\tests\config_test.rs canlink-tscan\src\lib.rs
git commit -m "feat: add tscan daemon config parsing"
```

### Task 2: JSON 协议结构定义

**Files:**
- Create: `canlink-tscan\src\daemon\protocol.rs`
- Test: `canlink-tscan\tests\daemon_protocol_test.rs`

- [ ] **Step 1: 写协议序列化测试（失败）**

```rust
#[test]
fn protocol_hello_roundtrip() {
    let req = Request::new(1, Op::Hello { protocol_version: 1, client_version: "client".into() });
    let json = serde_json::to_string(&req).unwrap();
    let back: Request = serde_json::from_str(&json).unwrap();
    assert_eq!(back, req);
}

#[test]
fn protocol_hello_ack_roundtrip() {
    let ack = HelloAck { protocol_version: 1, daemon_version: "d1".into() };
    let resp = Response::ok(1, serde_json::to_value(ack).unwrap());
    let json = serde_json::to_string(&resp).unwrap();
    let back: Response = serde_json::from_str(&json).unwrap();
    assert_eq!(back, resp);
}
```

- [ ] **Step 2: 运行测试并确认失败**

Run: `cargo test -p canlink-tscan --test daemon_protocol_test`
Expected: FAIL（类型不存在）

- [ ] **Step 3: 定义请求/响应与错误码**

```rust
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Request {
    pub id: u64,
    #[serde(flatten)]
    pub op: Op,
}

impl Request {
    pub fn new(id: u64, op: Op) -> Self { Self { id, op } }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "op", content = "params")]
pub enum Op {
    #[serde(rename = "HELLO")]
    Hello { protocol_version: u32, client_version: String },
    #[serde(rename = "INIT_LIB")]
    InitLib { enable_fifo: bool, enable_error_frame: bool, use_hw_time: bool },
    #[serde(rename = "SCAN")]
    Scan,
    #[serde(rename = "GET_DEVICE_INFO")]
    GetDeviceInfo { index: u32 },
    #[serde(rename = "CONNECT")]
    Connect { serial: String },
    #[serde(rename = "DISCONNECT_BY_HANDLE")]
    DisconnectByHandle { handle: u64 },
    #[serde(rename = "DISCONNECT_ALL")]
    DisconnectAll,
    #[serde(rename = "OPEN_CHANNEL")]
    OpenChannel { handle: u64, channel: u8 },
    #[serde(rename = "CLOSE_CHANNEL")]
    CloseChannel { handle: u64, channel: u8 },
    #[serde(rename = "CONFIG_CAN_BAUDRATE")]
    ConfigCanBaudrate { handle: u64, channel: u8, rate_kbps: f64, term: u8 },
    #[serde(rename = "CONFIG_CANFD_BAUDRATE")]
    ConfigCanfdBaudrate { handle: u64, channel: u8, arb_kbps: f64, data_kbps: f64, ctrl_type: u8, ctrl_mode: u8, term: u8 },
    #[serde(rename = "SEND_CAN")]
    SendCan { handle: u64, channel: u8, id: u32, is_ext: bool, data: Vec<u8> },
    #[serde(rename = "SEND_CANFD")]
    SendCanfd { handle: u64, channel: u8, id: u32, is_ext: bool, brs: bool, esi: bool, data: Vec<u8> },
    #[serde(rename = "RECV_CAN")]
    RecvCan { handle: u64, channel: u8, max_count: u8, timeout_ms: u64 },
    #[serde(rename = "RECV_CANFD")]
    RecvCanfd { handle: u64, channel: u8, max_count: u8, timeout_ms: u64 },
    #[serde(rename = "GET_CAPABILITY")]
    GetCapability { handle: u64 },
    #[serde(rename = "FINALIZE")]
    Finalize,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct HelloAck {
    pub protocol_version: u32,
    pub daemon_version: String,
}
// HELLO 响应的 data 必须是 HelloAck（包含 protocol_version）

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ok,
    Error,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Response {
    pub id: u64,
    pub status: Status,
    pub code: u32,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub data: serde_json::Value,
}

#[repr(u32)]
pub enum ErrorCode {
    Ok = 0,
    LibTscanError = 1,
    InvalidParams = 2,
    AlreadyConnected = 3,
    InvalidHandle = 4,
    InvalidChannel = 5,
    NoDevice = 6,
    InvalidIndex = 7,
    DeviceBusy = 8,
    ProtocolError = 9,
}

impl Response {
    pub fn ok(id: u64, data: serde_json::Value) -> Self { /* status=Ok, code=0 */ }
    pub fn error(id: u64, code: u32, message: impl Into<String>) -> Self { /* status=Error */ }
}
```

- [ ] **Step 4: 运行测试并确认通过**

Run: `cargo test -p canlink-tscan --test daemon_protocol_test`
Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add canlink-tscan\src\daemon\protocol.rs canlink-tscan\tests\daemon_protocol_test.rs
git commit -m "feat: define tscan daemon json protocol"
```

### Task 3: 长度前缀 JSON codec

**Files:**
- Create: `canlink-tscan\src\daemon\codec.rs`
- Modify: `canlink-tscan\src\daemon\mod.rs`
- Test: `canlink-tscan\tests\daemon_protocol_test.rs`

- [ ] **Step 1: 添加 codec 失败测试**

```rust
#[test]
fn codec_roundtrip() {
    let req = Request::new(1, Op::Hello { protocol_version: 1, client_version: "client".into() });
    let mut buf = Vec::new();
    write_frame(&mut buf, &req).unwrap();
    let back: Request = read_frame(&mut &buf[..]).unwrap();
    assert_eq!(back, req);
}

#[test]
fn codec_rejects_oversize_read() {
    let mut buf = Vec::new();
    buf.extend_from_slice(&((MAX_FRAME_SIZE as u32) + 1).to_le_bytes());
    let err = read_frame::<_, Request>(&mut &buf[..]).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn codec_rejects_oversize_write() {
    let big = "a".repeat(MAX_FRAME_SIZE + 1);
    let req = Request::new(1, Op::Hello { protocol_version: 1, client_version: big });
    let mut buf = Vec::new();
    assert!(write_frame(&mut buf, &req).is_err());
}
```

- [ ] **Step 2: 运行测试并确认失败**

Run: `cargo test -p canlink-tscan --test daemon_protocol_test`
Expected: FAIL

- [ ] **Step 3: 实现 length-prefixed codec**

```rust
pub const MAX_FRAME_SIZE: usize = 1024 * 1024;

pub fn write_frame<W: Write, T: Serialize>(w: &mut W, value: &T) -> io::Result<()> {
    let data = serde_json::to_vec(value)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    if data.len() > MAX_FRAME_SIZE {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "frame too large"));
    }
    let len = data.len() as u32;
    w.write_all(&len.to_le_bytes())?;
    w.write_all(&data)?;
    Ok(())
}

pub fn read_frame<R: Read, T: DeserializeOwned>(r: &mut R) -> io::Result<T> {
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len > MAX_FRAME_SIZE {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "frame too large"));
    }
    let mut data = vec![0u8; len];
    r.read_exact(&mut data)?;
    serde_json::from_slice(&data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
```

- [ ] **Step 4: 运行测试并确认通过**

Run: `cargo test -p canlink-tscan --test daemon_protocol_test`
Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add canlink-tscan\src\daemon\codec.rs canlink-tscan\src\daemon\mod.rs canlink-tscan\tests\daemon_protocol_test.rs
git commit -m "feat: add length-prefixed json codec"
```

---## Chunk 2: daemon 服务端与二进制入口

### Task 4: daemon 服务端骨架

**Files:**
- Create: `canlink-tscan\src\daemon\server.rs`
- Modify: `canlink-tscan\src\daemon\mod.rs`
- Test: `canlink-tscan\tests\daemon_server_test.rs`

- [ ] **Step 1: 写最小请求处理循环（返回统一错误）**

```rust
pub fn run_server() -> io::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    run_server_with_io(stdin.lock(), stdout.lock())
}

pub fn run_server_with_io<R: Read, W: Write>(mut reader: R, mut writer: W) -> io::Result<()> {
    loop {
        let req: Request = match read_frame(&mut reader) {
            Ok(v) => v,
            Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(err) => return Err(err),
        };
        let resp = Response::error(req.id, ErrorCode::ProtocolError as u32, "unsupported op");
        write_frame(&mut writer, &resp)?;
    }
}
```

- [ ] **Step 2: 添加单元级 smoke 测试（可复用后续 stub）**

```rust
#[test]
fn run_server_exits_on_eof() {
    let input = std::io::Cursor::new(Vec::<u8>::new());
    let mut output = Vec::new();
    run_server_with_io(input, &mut output).unwrap();
    assert!(output.is_empty());
}
```

- [ ] **Step 3: 提交**

```bash
git add canlink-tscan\src\daemon\server.rs canlink-tscan\src\daemon\mod.rs canlink-tscan\tests\daemon_server_test.rs
git commit -m "feat: scaffold tscan daemon server"
```

### Task 5: 实现核心 op 映射（HELLO/INIT/SCAN/CONNECT/DISCONNECT）

**Files:**
- Modify: `canlink-tscan\src\daemon\server.rs`

- [ ] **Step 1: 添加请求到响应的映射表**

```rust
match request.op {
    Op::Hello { protocol_version, .. } => {
        let ack = HelloAck { protocol_version, daemon_version: env!("CARGO_PKG_VERSION").into() };
        Response::ok(request.id, serde_json::to_value(ack).unwrap())
    }
    Op::InitLib { enable_fifo, enable_error_frame, use_hw_time } => {
        unsafe { initialize_lib_tscan(enable_fifo, enable_error_frame, use_hw_time); }
        Response::ok(request.id, json!({}))
    }
    Op::Scan => {
        let mut count: u32 = 0;
        let rc = unsafe { tscan_scan_devices(&mut count) };
        if rc != 0 {
            return Response::error(request.id, ErrorCode::LibTscanError as u32, format!("tscan_scan_devices: {rc}"));
        }
        let mut devices = Vec::new();
        for idx in 0..count {
            let mut manufacturer = std::ptr::null();
            let mut product = std::ptr::null();
            let mut serial = std::ptr::null();
            let rc = unsafe { tscan_get_device_info(idx, &mut manufacturer, &mut product, &mut serial) };
            if rc != 0 {
                return Response::error(request.id, ErrorCode::LibTscanError as u32, format!("tscan_get_device_info: {rc}"));
            }
            let m = unsafe { CStr::from_ptr(manufacturer).to_string_lossy().into_owned() };
            let p = unsafe { CStr::from_ptr(product).to_string_lossy().into_owned() };
            let s = unsafe { CStr::from_ptr(serial).to_string_lossy().into_owned() };
            devices.push(json!({ "manufacturer": m, "product": p, "serial": s, "device_type": 0 }));
        }
        Response::ok(request.id, json!({ "devices": devices }))
    }
    Op::GetDeviceInfo { index } => {
        let mut count: u32 = 0;
        let rc = unsafe { tscan_scan_devices(&mut count) };
        if rc != 0 {
            return Response::error(request.id, ErrorCode::LibTscanError as u32, format!("tscan_scan_devices: {rc}"));
        }
        if index >= count {
            return Response::error(request.id, ErrorCode::InvalidIndex as u32, "invalid_index");
        }
        let mut manufacturer = std::ptr::null();
        let mut product = std::ptr::null();
        let mut serial = std::ptr::null();
        let rc = unsafe { tscan_get_device_info(index, &mut manufacturer, &mut product, &mut serial) };
        if rc != 0 {
            return Response::error(request.id, ErrorCode::LibTscanError as u32, format!("tscan_get_device_info: {rc}"));
        }
        let m = unsafe { CStr::from_ptr(manufacturer).to_string_lossy().into_owned() };
        let p = unsafe { CStr::from_ptr(product).to_string_lossy().into_owned() };
        let s = unsafe { CStr::from_ptr(serial).to_string_lossy().into_owned() };
        Response::ok(request.id, json!({ "manufacturer": m, "product": p, "serial": s, "device_type": 0 }))
    }
    Op::Connect { serial } => {
        let serial = if serial.is_empty() {
            let mut count: u32 = 0;
            let rc = unsafe { tscan_scan_devices(&mut count) };
            if rc != 0 {
                return Response::error(request.id, ErrorCode::LibTscanError as u32, format!("tscan_scan_devices: {rc}"));
            }
            if count == 0 {
                return Response::error(request.id, ErrorCode::NoDevice as u32, "no_device");
            }
            let mut manufacturer = std::ptr::null();
            let mut product = std::ptr::null();
            let mut serial_ptr = std::ptr::null();
            let rc = unsafe { tscan_get_device_info(0, &mut manufacturer, &mut product, &mut serial_ptr) };
            if rc != 0 {
                return Response::error(request.id, ErrorCode::LibTscanError as u32, format!("tscan_get_device_info: {rc}"));
            }
            unsafe { CStr::from_ptr(serial_ptr).to_string_lossy().into_owned() }
        } else {
            serial
        };
        let serial_c = std::ffi::CString::new(serial).unwrap();
        let mut handle: usize = 0;
        let rc = unsafe { tscan_connect(serial_c.as_ptr(), &mut handle) };
        if rc != 0 {
            return Response::error(request.id, ErrorCode::LibTscanError as u32, format!("tscan_connect: {rc}"));
        }
        let mut channel_count: s32 = 0;
        let mut supports_canfd: bool = false;
        let rc = unsafe { tscan_get_can_channel_count(handle, &mut channel_count, &mut supports_canfd) };
        if rc != 0 {
            return Response::error(request.id, ErrorCode::LibTscanError as u32, format!("tscan_get_can_channel_count: {rc}"));
        }
        Response::ok(request.id, json!({ "handle": handle as u64, "channel_count": channel_count, "supports_canfd": supports_canfd }))
    }
    Op::DisconnectByHandle { handle } => {
        let rc = unsafe { tscan_disconnect_by_handle(handle as usize) };
        if rc != 0 {
            return Response::error(request.id, ErrorCode::LibTscanError as u32, format!("tscan_disconnect_by_handle: {rc}"));
        }
        Response::ok(request.id, json!({}))
    }
    Op::DisconnectAll => {
        let rc = unsafe { tscan_disconnect_all_devices() };
        if rc != 0 {
            return Response::error(request.id, ErrorCode::LibTscanError as u32, format!("tscan_disconnect_all_devices: {rc}"));
        }
        Response::ok(request.id, json!({}))
    }
    _ => Response::error(request.id, ErrorCode::ProtocolError as u32, "unsupported op"),
}
```

- [ ] **Step 2: 运行 daemon 单元测试**

Run: `cargo test -p canlink-tscan --lib`
Expected: PASS

- [ ] **Step 3: 提交**

```bash
git add canlink-tscan\src\daemon\server.rs
git commit -m "feat: implement daemon core ops"
```

### Task 6: 二进制入口

**Files:**
- Create: `canlink-tscan\src\bin\canlink-tscan-daemon.rs`
- Modify: `canlink-tscan\Cargo.toml`

- [ ] **Step 1: 添加 bin 声明与入口**

```rust
fn main() {
    if let Err(err) = canlink_tscan::daemon::server::run_server() {
        eprintln!("daemon error: {err}");
        std::process::exit(1);
    }
}
```

- [ ] **Step 2: 构建确认**

Run: `cargo build -p canlink-tscan --bin canlink-tscan-daemon`
Expected: SUCCESS

- [ ] **Step 3: 提交**

```bash
git add canlink-tscan\src\bin\canlink-tscan-daemon.rs canlink-tscan\Cargo.toml
git commit -m "feat: add canlink-tscan-daemon binary"
```

---

## Chunk 3: 客户端、后端接入、测试与文档

### Task 7: daemon 客户端与重启恢复

**Files:**
- Create: `canlink-tscan\src\daemon\client.rs`
- Modify: `canlink-tscan\src\daemon\mod.rs`

- [ ] **Step 1: 定义 DaemonClient 结构、进程句柄与缓存状态**

```rust
pub struct DaemonClient {
    config: TscanDaemonConfig,
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    stdout: std::process::ChildStdout,
    codec: FrameCodec,
    next_id: u64,
    cache: BackendStateCache,
}

impl DaemonClient {
    pub fn connect(config: &TscanDaemonConfig, init: InitParams) -> CanResult<Self> {
        // spawn + hello + init
    }

    pub fn request(&mut self, op: Op, timeout: Duration) -> CanResult<Response> {
        // send + recv
    }

    pub fn shutdown(&mut self) {
        // kill child if alive
    }
}
```
  - **单一来源**：`BackendStateCache` 由 `DaemonClient` 独占维护（放在 `daemon::client` 或 `daemon::mod`），`TSCanBackend` 不再单独持有，避免双份状态与同步问题。

- [ ] **Step 2: 实现 spawn / HELLO / INIT 与超时选择**
  - daemon_path：优先使用 `config.daemon_path`，否则走默认发现
  - stdin/stdout 使用 `piped`，stderr 继承给父进程便于排查
  - `stdout` 只允许输出协议帧；日志/trace 必须写 `stderr` 或 `TRACE_PATH`
  - `HELLO` / `HELLO_ACK` 必须校验 `protocol_version`，不一致直接返回 `CanError::InitializationFailed`
  - timeout 规则：
    - `DISCONNECT_*` 使用 `disconnect_timeout_ms`，外层仍保留 `request_timeout_ms` 作为 watchdog（如需更长超时可同步调大 `request_timeout_ms`）
    - `RECV_*` 若调用方显式传入 `timeout_ms` 则使用该值，否则使用配置的 `recv_timeout_ms`；外层仍保留 `request_timeout_ms` 作为 watchdog
    - 其他操作使用 `request_timeout_ms`
    - `FINALIZE` 超时：直接 kill child，不再重启

- [ ] **Step 3: 实现超时重启与状态恢复**
  - 触发条件：IO/EOF、协议错误、超时
  - 行为：
    1. kill 子进程（忽略错误）
    2. 重启（最多 `restart_max_retries` 次）
    3. `HELLO` + `INIT_LIB`
    4. `recover_state(&cache)`：`SCAN` -> `CONNECT(serial)` -> `CONFIG_*` -> `OPEN_CHANNEL`
  - 成功后允许对**幂等请求**（`SCAN` / `GET_DEVICE_INFO` / `GET_CAPABILITY`）重试一次
  - 超过重试上限 -> `CanError::InitializationFailed`

- [ ] **Step 4: 断开超时的“验证式成功”路径**
  - 仅针对 `DISCONNECT_*` 超时：**优先进入本路径**，跳过通用“重启 + recover_state”逻辑
    1. kill 子进程
    2. 重启 + `HELLO` + `INIT_LIB`
    3. `SCAN` + `CONNECT` 目标 serial（空串则取首个设备并缓存实际 serial）
    4. 若 `CONNECT` 成功 -> 立即 kill 子进程并返回 Ok（设备已可重新连接）
    5. 若返回 `device_busy` / `already_connected` -> `CanError::InitializationFailed`

- [ ] **Step 5: 提交**

```bash
git add canlink-tscan\src\daemon\client.rs canlink-tscan\src\daemon\mod.rs
git commit -m "feat: add daemon client with restart"
```

### Task 8: TSCanBackend 接入 daemon

**Files:**
- Modify: `canlink-tscan\src\backend.rs`
- Modify: `canlink-tscan\src\lib.rs`

- [ ] **Step 1: 引入配置与后端模式**
  - `TSCanBackend` 新增 `daemon: Option<DaemonClient>`（不再单独保存 `BackendStateCache`）
  - `initialize()` 中解析 `TscanDaemonConfig::resolve`，`use_daemon=false` 走原 FFI 分支
  - `use_daemon=true` 时创建 `DaemonClient::connect` 并完成 `INIT_LIB`
  - 确认 `TscanDaemonConfig::resolve` 默认 `use_daemon=true`（与文档说明一致）

- [ ] **Step 2: 定义 BackendStateCache 与更新规则（由 DaemonClient 维护）**

```rust
#[derive(Default, Clone)]
struct BackendStateCache {
    init: InitParams, // enable_fifo / enable_error_frame / use_hw_time
    serial: Option<String>,
    handle: Option<u64>,
    channel_count: Option<u8>,
    supports_canfd: Option<bool>,
    can_baud: HashMap<u8, CanBaudrateConfig>,
    canfd_baud: HashMap<u8, CanFdBaudrateConfig>,
    opened_channels: HashSet<u8>,
}
```

  - `initialize`：缓存 `init` 参数
  - `connect`：缓存 `serial/handle/channel_count/supports_canfd`
  - `config_*`：按通道缓存波特率
  - `open_channel/close_channel`：同步 `opened_channels`
  - `disconnect/finalize`：清空 cache

- [ ] **Step 3: 路由现有 API 到 daemon 并同步缓存**
  - `scan_devices` -> `SCAN`
  - `get_device_info` -> `GET_DEVICE_INFO`
  - `connect` -> `CONNECT` 并缓存 `serial/handle/channel_count/supports_canfd`
  - `disconnect_*` -> `DISCONNECT_*`，成功后清空 cache + `daemon=None`
  - `open_channel` / `close_channel` / `config_*` -> 对应 op，成功后更新 cache
  - `send_*` / `receive_*` -> `SEND_*` / `RECV_*`（`RECV_*` 透传显式 `timeout_ms`，未指定则使用配置的 `recv_timeout_ms`）
  - `finalize` -> `FINALIZE`，无论结果都 kill daemon + 清空 cache
  - `response.status=error` 按 `ErrorCode` 映射到 `CanError`（在 client 内统一处理）
  - 当 `DaemonClient` 触发不可恢复错误（重启失败/协议错误/IO 退出）时，显式 `daemon=None` 并清空 cache

- [ ] **Step 4: 运行单元测试**

Run: `cargo test -p canlink-tscan --lib`
Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add canlink-tscan\src\backend.rs canlink-tscan\src\lib.rs
git commit -m "feat: route TSCanBackend through daemon"
```

### Task 9: stub daemon 与无硬件集成测试

**Files:**
- Create: `canlink-tscan\src\bin\canlink-tscan-daemon-stub.rs`
- Create: `canlink-tscan\tests\daemon_client_test.rs`

- [ ] **Step 1: 编写 stub daemon（支持 hang / trace）**
  - 复用 protocol + codec，`stdin/stdout` 走同样协议
  - 环境变量：
    - `HANG_ON_OP=DISCONNECT_BY_HANDLE|DISCONNECT_ALL|...`：匹配时不返回响应
    - `TRACE_PATH=...`：每次启动写入 `HELLO`/`OP:<name>`，用于测试断言重启
    - `PROTOCOL_VERSION=2`：用于制造版本不匹配
  - 默认行为：对 `HELLO/INIT_LIB/SCAN/CONNECT/GET_CAPABILITY/...` 返回固定 OK 响应
  - `stdout` 只输出协议帧，trace 写 `stderr`/`TRACE_PATH`
  - stub 复用 `FrameCodec`（1 MiB 帧限制已在 Chunk 1 覆盖，无需重复测试）

- [ ] **Step 2: 编写 daemon_client_test**
  - 通过 `std::env::var("CARGO_BIN_EXE_canlink-tscan-daemon-stub")` 获取 stub 路径
  - 构造 `TscanDaemonConfig`：`daemon_path=stub`、短超时、`restart_max_retries=1`
  - 断言用例：
    - `disconnect_timeout_triggers_restart`：设置 `HANG_ON_OP=DISCONNECT_BY_HANDLE`，期望返回 Ok，并在 `TRACE_PATH` 中出现 2 次 `HELLO`
    - `hello_version_mismatch_returns_error`：设置 `PROTOCOL_VERSION=2`，`initialize` 直接失败

- [ ] **Step 3: 运行测试并确认通过**

Run: `cargo test -p canlink-tscan --test daemon_client_test`
Expected: PASS

- [ ] **Step 4: 提交**

```bash
git add canlink-tscan\src\bin\canlink-tscan-daemon-stub.rs canlink-tscan\tests\daemon_client_test.rs
git commit -m "test: add stub daemon and client tests"
```

### Task 10: 文档更新

**Files:**
- Modify: `canlink-tscan\README.md`
- Modify: `docs\user-guide.md`

- [ ] **Step 1: README 增加隔离规避说明与构建要求**
  - 默认启用 `use_daemon=true`
  - 说明 `canlink-tscan-daemon` 需要随项目构建（或指定 `daemon_path`）
  - 明确这是“厂商 Bug 规避”，后续官方修复将移除/降级

- [ ] **Step 2: user-guide 增加配置示例与字段解释**

```toml
# canlink-tscan.toml
use_daemon = true
request_timeout_ms = 2000
disconnect_timeout_ms = 3000
restart_max_retries = 3
recv_timeout_ms = 0
# daemon_path = "C:/path/to/canlink-tscan-daemon.exe"
```

  - 解释 `daemon_path` 优先级与默认发现逻辑
  - 说明 `use_daemon=false` 会回退到直接 DLL 调用

- [ ] **Step 3: 提交**

```bash
git add canlink-tscan\README.md docs\user-guide.md
git commit -m "docs: document tscan daemon workaround"
```

---

# Plan Review Loop
- 该计划拆分为 3 个 Chunk，需逐 Chunk 评审通过后再进入下一 Chunk。
- 若评审发现问题，先修订计划文件再继续。

# 执行准备
- 该仓库**禁止使用 worktree**（按用户要求），直接在当前分支执行。
- 计划完成后再进入实现。

