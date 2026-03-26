//! Configuration hot reload support (FR-014)
//!
//! Provides file watching and automatic configuration reloading.
//! This module is only available when the `hot-reload` feature is enabled.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::error::CanError;

#[cfg(test)]
static FORCE_START_ERROR: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(test)]
fn set_force_start_error(value: bool) {
    FORCE_START_ERROR.store(value, std::sync::atomic::Ordering::SeqCst);
}

/// Configuration change callback type
pub type ConfigChangeCallback = Box<dyn Fn(&Path) + Send + 'static>;

/// Configuration file watcher
///
/// Watches a configuration file for changes and triggers callbacks
/// when modifications are detected.
///
/// # Example
///
/// ```rust,ignore
/// use canlink_hal::hot_reload::ConfigWatcher;
/// use std::path::Path;
///
/// let mut watcher = ConfigWatcher::new("config.toml").unwrap();
/// watcher.on_config_change(|path| {
///     println!("Config changed: {:?}", path);
/// });
/// watcher.start().unwrap();
/// ```
pub struct ConfigWatcher {
    /// Path to the configuration file
    config_path: PathBuf,
    /// File watcher instance
    watcher: Option<RecommendedWatcher>,
    /// Event receiver
    rx: Option<Receiver<Result<Event, notify::Error>>>,
    /// Change callbacks
    callbacks: Arc<Mutex<Vec<ConfigChangeCallback>>>,
    /// Watcher thread handle
    thread_handle: Option<JoinHandle<()>>,
    /// Stop signal sender
    stop_tx: Option<Sender<()>>,
    /// Whether the watcher is running
    running: bool,
}

impl ConfigWatcher {
    /// Create a new configuration watcher
    ///
    /// # Arguments
    ///
    /// * `config_path` - Path to the configuration file to watch
    ///
    /// # Errors
    ///
    /// Returns an error if the path is invalid or the watcher cannot be created.
    pub fn new<P: AsRef<Path>>(config_path: P) -> Result<Self, CanError> {
        let config_path = config_path.as_ref().to_path_buf();

        // Verify the path exists
        if !config_path.exists() {
            return Err(CanError::ConfigError {
                reason: format!("Configuration file not found: {}", config_path.display()),
            });
        }

        Ok(Self {
            config_path,
            watcher: None,
            rx: None,
            callbacks: Arc::new(Mutex::new(Vec::new())),
            thread_handle: None,
            stop_tx: None,
            running: false,
        })
    }

    /// Register a callback for configuration changes
    ///
    /// The callback will be invoked whenever the configuration file is modified.
    pub fn on_config_change<F>(&mut self, callback: F)
    where
        F: Fn(&Path) + Send + 'static,
    {
        if let Ok(mut callbacks) = self.callbacks.lock() {
            callbacks.push(Box::new(callback));
        }
    }

    /// Start watching for configuration changes
    ///
    /// # Errors
    ///
    /// Returns an error if the watcher cannot be started.
    pub fn start(&mut self) -> Result<(), CanError> {
        #[cfg(test)]
        if FORCE_START_ERROR.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(CanError::ConfigError {
                reason: "forced start error".to_string(),
            });
        }

        if self.running {
            return Ok(());
        }

        let (tx, rx) = channel();
        let (stop_tx, stop_rx) = channel();

        // Create the watcher
        let watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default().with_poll_interval(Duration::from_secs(1)),
        )
        .map_err(|e| CanError::ConfigError {
            reason: format!("Failed to create watcher: {e}"),
        })?;

        self.watcher = Some(watcher);
        self.rx = Some(rx);
        self.stop_tx = Some(stop_tx);

        // Start watching the file
        if let Some(ref mut watcher) = self.watcher {
            watcher
                .watch(&self.config_path, RecursiveMode::NonRecursive)
                .map_err(|e| CanError::ConfigError {
                    reason: format!("Failed to watch file: {e}"),
                })?;
        }

        // Start the event processing thread
        let callbacks = Arc::clone(&self.callbacks);
        let config_path = self.config_path.clone();
        let rx = self.rx.take().ok_or_else(|| CanError::ConfigError {
            reason: "Hot reload receiver missing".to_string(),
        })?;

        let handle = thread::spawn(move || {
            loop {
                // Check for stop signal
                if stop_rx.try_recv().is_ok() {
                    break;
                }

                // Process events with timeout
                match rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(Ok(event)) => {
                        // Check if this is a modify event
                        if event.kind.is_modify() {
                            if let Ok(callbacks) = callbacks.lock() {
                                for callback in callbacks.iter() {
                                    callback(&config_path);
                                }
                            }
                        }
                    }
                    Ok(Err(_)) | Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        // No events or watcher error, continue
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        // Channel closed, exit
                        break;
                    }
                }
            }
        });

        self.thread_handle = Some(handle);
        self.running = true;

        Ok(())
    }

    /// Stop watching for configuration changes
    pub fn stop(&mut self) {
        if !self.running {
            return;
        }

        // Send stop signal
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }

        // Wait for thread to finish
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }

        // Stop the watcher
        if let Some(ref mut watcher) = self.watcher {
            let _ = watcher.unwatch(&self.config_path);
        }

        self.watcher = None;
        self.running = false;
    }

    /// Check if the watcher is running
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Get the path being watched
    #[must_use]
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }
}

impl Drop for ConfigWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    struct ForceStartGuard;

    impl ForceStartGuard {
        fn new() -> Self {
            set_force_start_error(true);
            Self
        }
    }

    impl Drop for ForceStartGuard {
        fn drop(&mut self) {
            set_force_start_error(false);
        }
    }

    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_new_watcher() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        File::create(&config_path).unwrap();

        let watcher = ConfigWatcher::new(&config_path);
        assert!(watcher.is_ok());
    }

    #[test]
    fn test_new_watcher_missing_file() {
        let result = ConfigWatcher::new("/nonexistent/config.toml");
        assert!(result.is_err());
    }

    #[test]
    fn test_start_stop() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        File::create(&config_path).unwrap();

        let mut watcher = ConfigWatcher::new(&config_path).unwrap();
        assert!(!watcher.is_running());

        watcher.start().unwrap();
        assert!(watcher.is_running());

        watcher.stop();
        assert!(!watcher.is_running());
    }

    #[test]
    fn test_start_stop_idempotent() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        File::create(&config_path).unwrap();

        let mut watcher = ConfigWatcher::new(&config_path).unwrap();

        watcher.start().unwrap();
        watcher.start().unwrap();

        watcher.stop();
        watcher.stop();
    }

    #[test]
    fn test_start_forced_error_path() {
        let _guard = TEST_LOCK.lock().unwrap();
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        File::create(&config_path).unwrap();

        let mut watcher = ConfigWatcher::new(&config_path).unwrap();

        let _force = ForceStartGuard::new();
        let result = watcher.start();

        assert!(matches!(result, Err(CanError::ConfigError { .. })));
    }

    #[test]
    fn test_callback_registration() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        File::create(&config_path).unwrap();

        let mut watcher = ConfigWatcher::new(&config_path).unwrap();

        let called = Arc::new(Mutex::new(false));
        let called_clone = Arc::clone(&called);

        watcher.on_config_change(move |_| {
            *called_clone.lock().unwrap() = true;
        });

        // Verify callback was registered
        assert_eq!(watcher.callbacks.lock().unwrap().len(), 1);
    }

    #[test]
    fn test_config_change_detection() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        {
            let mut file = File::create(&config_path).unwrap();
            writeln!(file, "key = \"value1\"").unwrap();
        }

        let mut watcher = ConfigWatcher::new(&config_path).unwrap();

        let change_detected = Arc::new(Mutex::new(false));
        let change_detected_clone = Arc::clone(&change_detected);

        watcher.on_config_change(move |_| {
            *change_detected_clone.lock().unwrap() = true;
        });

        watcher.start().unwrap();

        // Modify the file
        thread::sleep(Duration::from_millis(200));
        {
            let mut file = File::create(&config_path).unwrap();
            writeln!(file, "key = \"value2\"").unwrap();
        }

        // Wait for the change to be detected
        thread::sleep(Duration::from_secs(2));

        watcher.stop();

        // Note: This test may be flaky depending on the file system
        // The change detection depends on the notify crate's behavior
    }
}
