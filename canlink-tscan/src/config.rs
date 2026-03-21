//! Configuration for the TSCan daemon workaround.

use canlink_hal::{BackendConfig, CanError, CanResult};
use serde::Deserialize;
use std::path::Path;

const DEFAULT_CONFIG_FILE: &str = "canlink-tscan.toml";
const DEFAULT_USE_DAEMON: bool = true;
const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 2000;
const DEFAULT_DISCONNECT_TIMEOUT_MS: u64 = 3000;
const DEFAULT_RESTART_MAX_RETRIES: u32 = 3;
const DEFAULT_RECV_TIMEOUT_MS: u64 = 0;

#[derive(Debug, Clone, Deserialize, Default)]
/// File-based daemon configuration loaded from `canlink-tscan.toml`.
pub struct FileConfig {
    /// Whether to enable daemon mode.
    pub use_daemon: Option<bool>,
    /// Optional daemon executable path.
    pub daemon_path: Option<String>,
    /// Request timeout in milliseconds.
    pub request_timeout_ms: Option<u64>,
    /// Disconnect timeout in milliseconds.
    pub disconnect_timeout_ms: Option<u64>,
    /// Maximum restart retries after daemon failure.
    pub restart_max_retries: Option<u32>,
    /// Receive timeout in milliseconds.
    pub recv_timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Effective daemon configuration merged from file and backend parameters.
pub struct TscanDaemonConfig {
    /// Whether to use daemon mode instead of direct DLL calls.
    pub use_daemon: bool,
    /// Optional daemon executable path.
    pub daemon_path: Option<String>,
    /// Request timeout in milliseconds.
    pub request_timeout_ms: u64,
    /// Disconnect timeout in milliseconds.
    pub disconnect_timeout_ms: u64,
    /// Maximum restart retries after daemon failure.
    pub restart_max_retries: u32,
    /// Receive timeout in milliseconds.
    pub recv_timeout_ms: u64,
}

impl Default for TscanDaemonConfig {
    fn default() -> Self {
        Self {
            use_daemon: DEFAULT_USE_DAEMON,
            daemon_path: None,
            request_timeout_ms: DEFAULT_REQUEST_TIMEOUT_MS,
            disconnect_timeout_ms: DEFAULT_DISCONNECT_TIMEOUT_MS,
            restart_max_retries: DEFAULT_RESTART_MAX_RETRIES,
            recv_timeout_ms: DEFAULT_RECV_TIMEOUT_MS,
        }
    }
}

impl TscanDaemonConfig {
    /// Loads optional file configuration from `path`.
    pub fn load_file(path: &Path) -> CanResult<Option<FileConfig>> {
        if !path.exists() {
            return Ok(None);
        }

        let text = std::fs::read_to_string(path).map_err(|err| CanError::ConfigError {
            reason: format!("failed to read '{}': {err}", path.display()),
        })?;
        let parsed: FileConfig = toml::from_str(&text).map_err(|err| CanError::ConfigError {
            reason: format!("failed to parse '{}': {err}", path.display()),
        })?;
        Ok(Some(parsed))
    }

    /// Resolves final configuration from defaults, config file and backend parameters.
    pub fn resolve(backend: &BackendConfig) -> CanResult<Self> {
        let mut merged = Self::default();

        if let Some(file_cfg) = Self::load_file(Path::new(DEFAULT_CONFIG_FILE))? {
            merged.apply_file(&file_cfg);
        }
        merged.apply_backend_config(backend)?;
        Ok(merged)
    }

    fn apply_file(&mut self, cfg: &FileConfig) {
        if let Some(value) = cfg.use_daemon {
            self.use_daemon = value;
        }
        if let Some(value) = &cfg.daemon_path {
            self.daemon_path = Some(value.clone());
        }
        if let Some(value) = cfg.request_timeout_ms {
            self.request_timeout_ms = value;
        }
        if let Some(value) = cfg.disconnect_timeout_ms {
            self.disconnect_timeout_ms = value;
        }
        if let Some(value) = cfg.restart_max_retries {
            self.restart_max_retries = value;
        }
        if let Some(value) = cfg.recv_timeout_ms {
            self.recv_timeout_ms = value;
        }
    }

    fn apply_backend_config(&mut self, cfg: &BackendConfig) -> CanResult<()> {
        if let Some(value) = read_bool(cfg, "use_daemon")? {
            self.use_daemon = value;
        }
        if let Some(value) = read_string(cfg, "daemon_path")? {
            self.daemon_path = Some(value);
        }
        if let Some(value) = read_u64(cfg, "request_timeout_ms")? {
            self.request_timeout_ms = value;
        }
        if let Some(value) = read_u64(cfg, "disconnect_timeout_ms")? {
            self.disconnect_timeout_ms = value;
        }
        if let Some(value) = read_u32(cfg, "restart_max_retries")? {
            self.restart_max_retries = value;
        }
        if let Some(value) = read_u64(cfg, "recv_timeout_ms")? {
            self.recv_timeout_ms = value;
        }
        Ok(())
    }
}

fn read_bool(cfg: &BackendConfig, key: &str) -> CanResult<Option<bool>> {
    match cfg.parameters.get(key) {
        None => Ok(None),
        Some(value) => value.as_bool().map(Some).ok_or(CanError::InvalidParameter {
            parameter: key.to_string(),
            reason: "expected boolean".to_string(),
        }),
    }
}

fn read_string(cfg: &BackendConfig, key: &str) -> CanResult<Option<String>> {
    match cfg.parameters.get(key) {
        None => Ok(None),
        Some(value) => {
            value
                .as_str()
                .map(|v| Some(v.to_string()))
                .ok_or(CanError::InvalidParameter {
                    parameter: key.to_string(),
                    reason: "expected string".to_string(),
                })
        }
    }
}

fn read_u64(cfg: &BackendConfig, key: &str) -> CanResult<Option<u64>> {
    match cfg.parameters.get(key) {
        None => Ok(None),
        Some(value) => {
            let raw = value.as_integer().ok_or(CanError::InvalidParameter {
                parameter: key.to_string(),
                reason: "expected integer".to_string(),
            })?;
            if raw < 0 {
                return Err(CanError::InvalidParameter {
                    parameter: key.to_string(),
                    reason: "must be >= 0".to_string(),
                });
            }
            Ok(Some(raw as u64))
        }
    }
}

fn read_u32(cfg: &BackendConfig, key: &str) -> CanResult<Option<u32>> {
    let value = read_u64(cfg, key)?;
    if let Some(v) = value {
        if v > u32::MAX as u64 {
            return Err(CanError::InvalidParameter {
                parameter: key.to_string(),
                reason: "out of range for u32".to_string(),
            });
        }
        return Ok(Some(v as u32));
    }
    Ok(None)
}
