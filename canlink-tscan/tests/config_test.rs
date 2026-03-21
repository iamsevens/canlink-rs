use canlink_hal::BackendConfig;
use canlink_tscan::TscanDaemonConfig;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

static CWD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

struct DirGuard {
    original: std::path::PathBuf,
}

impl Drop for DirGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.original);
    }
}

fn unique_temp_dir() -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time is before unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("canlink-tscan-config-test-{nanos}"))
}

#[test]
fn config_precedence_backend_over_file_over_default() {
    let lock = CWD_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();

    let temp = unique_temp_dir();
    std::fs::create_dir_all(&temp).expect("create temp dir failed");
    std::fs::write(
        temp.join("canlink-tscan.toml"),
        r#"
use_daemon = true
request_timeout_ms = 2222
disconnect_timeout_ms = 3333
"#,
    )
    .expect("write config failed");

    let original = std::env::current_dir().expect("get current dir failed");
    let _guard = DirGuard { original };
    std::env::set_current_dir(&temp).expect("set current dir failed");

    let mut backend = BackendConfig::new("tscan");
    backend
        .parameters
        .insert("use_daemon".into(), toml::Value::Boolean(false));
    backend
        .parameters
        .insert("request_timeout_ms".into(), toml::Value::Integer(1111));

    let merged = TscanDaemonConfig::resolve(&backend).expect("resolve config failed");
    assert!(!merged.use_daemon);
    assert_eq!(merged.request_timeout_ms, 1111);
    assert_eq!(merged.disconnect_timeout_ms, 3333);
    assert_eq!(merged.restart_max_retries, 3);
    assert_eq!(merged.recv_timeout_ms, 0);

    drop(lock);
}

#[test]
fn config_invalid_backend_parameter_type_is_rejected() {
    let mut backend = BackendConfig::new("tscan");
    backend.parameters.insert(
        "request_timeout_ms".into(),
        toml::Value::String("not-number".to_string()),
    );

    let result = TscanDaemonConfig::resolve(&backend);
    assert!(result.is_err());
}
