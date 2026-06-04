use canlink_hal::{BackendConfig, CanBackend};
use canlink_tscan::TSCanBackend;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn unique_trace_file() -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time error")
        .as_nanos();
    std::env::temp_dir().join(format!("canlink-tscan-backend-batch-trace-{nanos}.log"))
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

#[test]
fn receive_message_drains_batch_and_serves_pending_frames() {
    let _guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|err| err.into_inner());

    let trace_path = unique_trace_file();
    std::env::set_var("TRACE_PATH", &trace_path);
    std::env::set_var("STUB_RECV_CAN_MESSAGES", "3");
    std::env::remove_var("EXIT_ON_OP_ONCE");
    std::env::remove_var("HANG_ON_OP");
    std::env::remove_var("PROTOCOL_VERSION");

    let mut config = BackendConfig::new("tscan");
    config
        .parameters
        .insert("use_daemon".into(), toml::Value::Boolean(true));
    config.parameters.insert(
        "daemon_path".into(),
        toml::Value::String(stub_daemon_path()),
    );
    config
        .parameters
        .insert("recv_batch_size".into(), toml::Value::Integer(3));

    let mut backend = TSCanBackend::new();
    backend.initialize(&config).expect("initialize backend");
    backend.open_channel(0).expect("open channel");

    let first = backend
        .receive_message()
        .expect("first receive")
        .expect("first message");
    let second = backend
        .receive_message()
        .expect("second receive")
        .expect("second message");

    backend.close().expect("close backend");
    std::env::remove_var("TRACE_PATH");
    std::env::remove_var("STUB_RECV_CAN_MESSAGES");

    assert_eq!(first.id().raw(), 0x100);
    assert_eq!(second.id().raw(), 0x101);

    let trace = std::fs::read_to_string(trace_path).expect("read trace file failed");
    let recv_can_count = trace
        .lines()
        .filter(|line| line.trim() == "OP:RECV_CAN")
        .count();
    assert_eq!(
        recv_can_count, 1,
        "expected second receive to use cached pending frame, trace was:\n{trace}"
    );
}
