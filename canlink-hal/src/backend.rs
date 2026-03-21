//! Backend trait definitions and factory pattern.
//!
//! This module defines the core `CanBackend` trait that all hardware backends must implement,
//! as well as the `BackendFactory` trait for creating backend instances.

use crate::{BackendConfig, BackendVersion, CanError, CanMessage, CanResult, HardwareCapability};
use std::time::Duration;

/// CAN hardware backend interface.
///
/// This trait defines the unified interface that all hardware backends must implement.
/// It provides methods for lifecycle management, message transmission/reception,
/// channel management, and capability querying.
///
/// # Thread Safety
///
/// This trait's methods require external synchronization. If you need to access the same
/// backend instance from multiple threads, the caller must provide synchronization using
/// `Mutex` or `RwLock`.
///
/// **Rationale**: External synchronization allows high-performance single-threaded usage
/// without lock overhead, while still supporting multi-threaded scenarios when needed.
///
/// ## Single-Threaded Usage (No Synchronization Needed)
///
/// ```rust,ignore
/// use canlink_hal::{CanBackend, BackendConfig};
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut backend = create_backend();
///     backend.initialize(&config)?;
///     backend.open_channel(0)?;
///
///     // Direct access - no locks needed
///     backend.send_message(&message)?;
///     if let Some(msg) = backend.receive_message()? {
///         println!("Received: {:?}", msg);
///     }
///
///     backend.close()?;
///     Ok(())
/// }
/// ```
///
/// ## Multi-Threaded Usage with Mutex
///
/// Use `Arc<Mutex<>>` when multiple threads need mutable access:
///
/// ```rust,ignore
/// use std::sync::{Arc, Mutex};
/// use std::thread;
/// use canlink_hal::{CanBackend, CanMessage};
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let backend = Arc::new(Mutex::new(create_backend()));
///
///     // Initialize in main thread
///     backend.lock().unwrap().initialize(&config)?;
///     backend.lock().unwrap().open_channel(0)?;
///
///     // Sender thread
///     let backend_tx = Arc::clone(&backend);
///     let tx_handle = thread::spawn(move || {
///         for i in 0..100 {
///             let msg = CanMessage::new_standard(0x100 + i, &[i as u8]).unwrap();
///             backend_tx.lock().unwrap().send_message(&msg).unwrap();
///         }
///     });
///
///     // Receiver thread
///     let backend_rx = Arc::clone(&backend);
///     let rx_handle = thread::spawn(move || {
///         let mut count = 0;
///         while count < 100 {
///             if let Some(msg) = backend_rx.lock().unwrap().receive_message().unwrap() {
///                 println!("Received: {:?}", msg);
///                 count += 1;
///             }
///         }
///     });
///
///     tx_handle.join().unwrap();
///     rx_handle.join().unwrap();
///
///     backend.lock().unwrap().close()?;
///     Ok(())
/// }
/// ```
///
/// ## Multi-Threaded Usage with `RwLock`
///
/// Use `Arc<RwLock<>>` when you have many readers and few writers:
///
/// ```rust,ignore
/// use std::sync::{Arc, RwLock};
/// use std::thread;
/// use canlink_hal::CanBackend;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let backend = Arc::new(RwLock::new(create_backend()));
///
///     // Initialize
///     backend.write().unwrap().initialize(&config)?;
///     backend.write().unwrap().open_channel(0)?;
///
///     // Multiple reader threads querying capabilities
///     let mut handles = vec![];
///     for i in 0..10 {
///         let backend_clone = Arc::clone(&backend);
///         let handle = thread::spawn(move || {
///             // Read lock allows concurrent access
///             let capability = backend_clone.read().unwrap().get_capability().unwrap();
///             println!("Thread {}: {} channels", i, capability.channel_count);
///         });
///         handles.push(handle);
///     }
///
///     // Writer thread sending messages
///     let backend_writer = Arc::clone(&backend);
///     let writer_handle = thread::spawn(move || {
///         for i in 0..10 {
///             let msg = CanMessage::new_standard(0x200, &[i]).unwrap();
///             // Write lock for exclusive access
///             backend_writer.write().unwrap().send_message(&msg).unwrap();
///         }
///     });
///
///     for handle in handles {
///         handle.join().unwrap();
///     }
///     writer_handle.join().unwrap();
///
///     backend.write().unwrap().close()?;
///     Ok(())
/// }
/// ```
///
/// ## Channel-Based Message Passing Pattern
///
/// For better performance, consider using channels to decouple threads:
///
/// ```rust,ignore
/// use std::sync::mpsc;
/// use std::thread;
/// use canlink_hal::{CanBackend, CanMessage};
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut backend = create_backend();
///     backend.initialize(&config)?;
///     backend.open_channel(0)?;
///
///     let (tx, rx) = mpsc::channel();
///
///     // Worker threads send messages via channel
///     for i in 0..4 {
///         let tx_clone = tx.clone();
///         thread::spawn(move || {
///             for j in 0..25 {
///                 let msg = CanMessage::new_standard(
///                     0x100 + (i * 25 + j),
///                     &[i as u8, j as u8]
///                 ).unwrap();
///                 tx_clone.send(msg).unwrap();
///             }
///         });
///     }
///     drop(tx); // Close sender
///
///     // Main thread owns backend and sends messages
///     for msg in rx {
///         backend.send_message(&msg)?;
///     }
///
///     backend.close()?;
///     Ok(())
/// }
/// ```
///
/// ## Performance Considerations
///
/// - **Single-threaded**: Zero synchronization overhead
/// - **Mutex**: Simple but serializes all access
/// - **`RwLock`**: Better for read-heavy workloads (e.g., capability queries)
/// - **Channel-based**: Best performance, avoids lock contention
///
/// ## Thread Safety Guarantees
///
/// - `CanBackend` is `Send`, so it can be moved between threads
/// - Methods require `&mut self`, enforcing exclusive access
/// - Backends do not use internal locks (external synchronization model)
/// - All backend state is protected by the caller's synchronization primitive
///
/// # Lifecycle
///
/// Backend instances follow this lifecycle:
/// 1. **Create** (via `BackendFactory::create()`)
/// 2. **Initialize** (`initialize()`)
/// 3. **Run** (call `send_message()`, `receive_message()`, etc.)
/// 4. **Close** (`close()`)
///
/// # Examples
///
/// ```rust,ignore
/// use canlink_hal::{CanBackend, BackendConfig};
///
/// // Create and initialize backend
/// let mut backend = create_backend();
/// backend.initialize(&config)?;
///
/// // Use backend
/// backend.open_channel(0)?;
/// backend.send_message(&message)?;
///
/// // Clean up
/// backend.close()?;
/// ```
pub trait CanBackend: Send {
    // ========== Lifecycle Management ==========

    /// Initialize the backend.
    ///
    /// # Arguments
    ///
    /// * `config` - Backend configuration parameters
    ///
    /// # Errors
    ///
    /// * `CanError::InitializationFailed` - Hardware initialization failed
    /// * `CanError::ConfigError` - Configuration parameters are invalid
    ///
    /// # Preconditions
    ///
    /// * Backend is in `Uninitialized` state
    ///
    /// # Postconditions
    ///
    /// * Success: Backend is in `Ready` state
    /// * Failure: Backend remains in `Uninitialized` state
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut backend = create_backend();
    /// backend.initialize(&config)?;
    /// ```
    fn initialize(&mut self, config: &BackendConfig) -> CanResult<()>;

    /// Close the backend and release resources.
    ///
    /// This method releases all resources held by the backend, including hardware handles,
    /// memory buffers, and network connections. It should be called when the backend is
    /// no longer needed.
    ///
    /// # Errors
    ///
    /// Returns an error if closing fails, but resources will still be released on a
    /// best-effort basis.
    ///
    /// # Preconditions
    ///
    /// * Backend is in `Ready` state
    ///
    /// # Postconditions
    ///
    /// * Backend is in `Closed` state
    /// * All resources are released
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// backend.close()?;
    /// ```
    fn close(&mut self) -> CanResult<()>;

    // ========== Hardware Capability Query ==========

    /// Query hardware capabilities.
    ///
    /// Returns information about the hardware's capabilities, including supported
    /// channel count, CAN-FD support, maximum bitrate, and timestamp precision.
    ///
    /// Applications should query capabilities at startup to adapt their behavior
    /// to the available hardware features. This enables writing portable code that
    /// works across different CAN hardware backends.
    ///
    /// # Performance Requirements
    ///
    /// * Response time < 1ms (SC-004)
    /// * Should be cached by the backend for fast repeated queries
    ///
    /// # Use Cases
    ///
    /// * **Feature Detection**: Check if CAN-FD is supported before sending FD frames
    /// * **Channel Validation**: Verify channel numbers before opening
    /// * **Bitrate Selection**: Choose from supported bitrates
    /// * **Filter Planning**: Determine available hardware filters
    /// * **Timestamp Handling**: Adapt to available timestamp precision
    ///
    /// # Examples
    ///
    /// Basic capability query:
    ///
    /// ```rust,ignore
    /// let capability = backend.get_capability()?;
    /// println!("Hardware: {} channels, CAN-FD: {}",
    ///          capability.channel_count,
    ///          capability.supports_canfd);
    /// ```
    ///
    /// Adaptive message sending:
    ///
    /// ```rust,ignore
    /// let capability = backend.get_capability()?;
    ///
    /// // Use CAN-FD if available, otherwise fall back to CAN 2.0
    /// let message = if capability.supports_canfd {
    ///     CanMessage::new_canfd(0x123, &data, false)?
    /// } else {
    ///     CanMessage::new_standard(0x123, &data[..8])?
    /// };
    /// backend.send_message(&message)?;
    /// ```
    ///
    /// Bitrate validation:
    ///
    /// ```rust,ignore
    /// let capability = backend.get_capability()?;
    /// let desired_bitrate = 500_000;
    ///
    /// if !capability.supports_bitrate(desired_bitrate) {
    ///     eprintln!("Bitrate {} not supported", desired_bitrate);
    ///     eprintln!("Supported bitrates: {:?}", capability.supported_bitrates);
    ///     return Err(CanError::UnsupportedFeature("bitrate".into()));
    /// }
    /// ```
    ///
    /// Channel validation:
    ///
    /// ```rust,ignore
    /// let capability = backend.get_capability()?;
    /// let channel = 2;
    ///
    /// if !capability.has_channel(channel) {
    ///     return Err(CanError::InvalidChannel(channel));
    /// }
    /// backend.open_channel(channel)?;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the capability query fails (rare, typically only on
    /// uninitialized or closed backends).
    fn get_capability(&self) -> CanResult<HardwareCapability>;

    // ========== Message Transmission/Reception ==========

    /// Send a CAN message.
    ///
    /// Transmits a CAN message on the bus. The message must be valid and the hardware
    /// must support the message type (e.g., CAN-FD messages require CAN-FD support).
    ///
    /// # Arguments
    ///
    /// * `message` - The message to send
    ///
    /// # Errors
    ///
    /// * `CanError::SendFailed` - Send failed (e.g., bus Bus-Off)
    /// * `CanError::UnsupportedFeature` - Hardware doesn't support this message type
    /// * `CanError::InvalidDataLength` - Data length exceeds limits
    /// * `CanError::InvalidState` - Backend not in Ready state
    ///
    /// # Preconditions
    ///
    /// * Backend is in `Ready` state
    /// * Message format is valid
    /// * At least one channel is open
    ///
    /// # Postconditions
    ///
    /// * Success: Message is sent to the bus
    /// * Failure: Message is not sent, backend state unchanged
    ///
    /// # Performance Requirements
    ///
    /// * Support 1000 messages/second throughput
    /// * Abstraction layer overhead < 5% (SC-005)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let msg = CanMessage::new_standard(0x123, &[0x01, 0x02, 0x03])?;
    /// backend.send_message(&msg)?;
    /// ```
    fn send_message(&mut self, message: &CanMessage) -> CanResult<()>;

    /// Receive a CAN message (non-blocking).
    ///
    /// Attempts to receive a message from the receive queue. Returns immediately
    /// with `None` if no messages are available.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(message))` - A message was received
    /// * `Ok(None)` - No messages currently available
    /// * `Err(CanError)` - Reception failed
    ///
    /// # Errors
    ///
    /// * `CanError::ReceiveFailed` - Reception failed
    /// * `CanError::InvalidState` - Backend not in Ready state
    ///
    /// # Preconditions
    ///
    /// * Backend is in `Ready` state
    ///
    /// # Postconditions
    ///
    /// * Success: Returned message is removed from receive queue
    /// * Failure: Receive queue state unchanged
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// if let Some(msg) = backend.receive_message()? {
    ///     println!("Received: {:?}", msg);
    /// }
    /// ```
    fn receive_message(&mut self) -> CanResult<Option<CanMessage>>;

    // ========== Channel Management ==========

    /// Open a CAN channel.
    ///
    /// Opens the specified CAN channel for communication. The channel index must be
    /// valid (less than the channel count reported by `get_capability()`).
    ///
    /// # Arguments
    ///
    /// * `channel` - Channel index (0-based)
    ///
    /// # Errors
    ///
    /// * `CanError::ChannelNotFound` - Channel doesn't exist
    /// * `CanError::ChannelAlreadyOpen` - Channel is already open
    ///
    /// # Preconditions
    ///
    /// * Backend is in `Ready` state
    /// * Channel index is valid (< `capability.channel_count`)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// backend.open_channel(0)?;
    /// ```
    fn open_channel(&mut self, channel: u8) -> CanResult<()>;

    /// Close a CAN channel.
    ///
    /// Closes the specified CAN channel and stops communication on that channel.
    ///
    /// # Arguments
    ///
    /// * `channel` - Channel index
    ///
    /// # Errors
    ///
    /// * `CanError::ChannelNotFound` - Channel doesn't exist
    /// * `CanError::ChannelNotOpen` - Channel is not open
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// backend.close_channel(0)?;
    /// ```
    fn close_channel(&mut self, channel: u8) -> CanResult<()>;

    // ========== Version Information ==========

    /// Get the backend version.
    ///
    /// Returns the semantic version number of the backend implementation.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let version = backend.version();
    /// println!("Backend version: {}", version);
    /// ```
    fn version(&self) -> BackendVersion;

    /// Get the backend name.
    ///
    /// Returns the unique identifier name of the backend (e.g., "tsmaster", "mock").
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let name = backend.name();
    /// println!("Using backend: {}", name);
    /// ```
    fn name(&self) -> &str;
}

/// Backend factory trait.
///
/// This trait defines the factory pattern for creating backend instances.
/// Each backend implementation should provide a factory that implements this trait.
///
/// # Examples
///
/// ```rust,ignore
/// use canlink_hal::{BackendFactory, BackendConfig};
///
/// struct MockBackendFactory;
///
/// impl BackendFactory for MockBackendFactory {
///     fn create(&self, config: &BackendConfig) -> CanResult<Box<dyn CanBackend>> {
///         Ok(Box::new(MockBackend::new()))
///     }
///
///     fn name(&self) -> &str {
///         "mock"
///     }
///
///     fn version(&self) -> BackendVersion {
///         BackendVersion::new(0, 1, 0)
///     }
/// }
/// ```
pub trait BackendFactory: Send + Sync {
    /// Create a new backend instance.
    ///
    /// # Arguments
    ///
    /// * `config` - Backend configuration
    ///
    /// # Returns
    ///
    /// A boxed backend instance ready for initialization.
    ///
    /// # Errors
    ///
    /// * `CanError::ConfigError` - Invalid configuration
    /// * `CanError::Other` - Factory-specific errors
    fn create(&self, config: &BackendConfig) -> CanResult<Box<dyn CanBackend>>;

    /// Get the factory name.
    ///
    /// Returns the unique identifier for this backend type.
    fn name(&self) -> &str;

    /// Get the factory version.
    ///
    /// Returns the version of the backend implementation.
    fn version(&self) -> BackendVersion;
}

/// Helper function to retry backend initialization.
///
/// This function implements the retry logic specified in FR-009. It attempts to
/// initialize a backend multiple times with a fixed interval between attempts.
///
/// # Arguments
///
/// * `backend` - The backend to initialize
/// * `config` - Backend configuration
/// * `retry_count` - Number of retry attempts (default: 3)
/// * `retry_interval` - Interval between retries in milliseconds (default: 1000)
///
/// # Returns
///
/// * `Ok(())` - Initialization succeeded
/// * `Err(CanError::InitializationFailed)` - All retry attempts failed
///
/// # Errors
///
/// Returns `CanError::InitializationFailed` if all retry attempts fail.
///
/// # Examples
///
/// ```rust,ignore
/// let mut backend = create_backend();
/// retry_initialize(&mut backend, &config, 3, 1000)?;
/// ```
pub fn retry_initialize(
    backend: &mut dyn CanBackend,
    config: &BackendConfig,
    retry_count: u32,
    retry_interval_ms: u64,
) -> CanResult<()> {
    let mut errors = Vec::new();
    let start_time = std::time::Instant::now();

    for attempt in 0..=retry_count {
        match backend.initialize(config) {
            Ok(()) => return Ok(()),
            Err(e) => {
                errors.push(format!("Attempt {}: {}", attempt + 1, e));
                if attempt < retry_count {
                    std::thread::sleep(Duration::from_millis(retry_interval_ms));
                }
            }
        }
    }

    let total_time = start_time.elapsed();
    Err(CanError::InitializationFailed {
        reason: format!(
            "Failed after {} attempts in {:?}. Errors: {}",
            retry_count + 1,
            total_time,
            errors.join("; ")
        ),
    })
}

/// Switch from one backend to another (FR-015).
///
/// This function performs a clean switch between backends:
/// 1. Closes the old backend (discarding any unprocessed messages)
/// 2. Initializes the new backend
///
/// **Important**: Any messages in the old backend's queue are discarded.
/// Users should process all pending messages before calling this function
/// if message preservation is required.
///
/// # Arguments
///
/// * `old_backend` - The currently active backend to close
/// * `new_backend` - The new backend to initialize
/// * `config` - Configuration for the new backend
///
/// # Errors
///
/// Returns an error if:
/// - The old backend fails to close (warning only, continues)
/// - The new backend fails to initialize
///
/// # Examples
///
/// ```rust,ignore
/// use canlink_hal::{switch_backend, BackendConfig};
///
/// // Process any remaining messages first
/// while let Some(msg) = old_backend.receive_message()? {
///     process_message(msg);
/// }
///
/// // Then switch backends
/// switch_backend(&mut old_backend, &mut new_backend, &new_config)?;
/// ```
pub fn switch_backend(
    old_backend: &mut dyn CanBackend,
    new_backend: &mut dyn CanBackend,
    config: &BackendConfig,
) -> CanResult<()> {
    // Get names upfront to avoid borrow issues (used for logging)
    #[allow(unused_variables)]
    let old_name = old_backend.name().to_string();
    #[allow(unused_variables)]
    let new_name = new_backend.name().to_string();

    // Log the switch (if tracing is enabled)
    #[cfg(feature = "tracing")]
    tracing::info!("Switching backend from '{}' to '{}'", old_name, new_name);

    // Close the old backend - ignore errors but log them
    #[allow(unused_variables)]
    if let Err(e) = old_backend.close() {
        #[cfg(feature = "tracing")]
        tracing::warn!("Error closing old backend '{}': {}", old_name, e);
        // Continue anyway - we want to switch even if close fails
    }

    // Initialize the new backend
    #[cfg(feature = "tracing")]
    {
        new_backend.initialize(config).map_err(|e| {
            tracing::error!("Failed to initialize new backend '{}': {}", new_name, e);
            e
        })?;
    }
    #[cfg(not(feature = "tracing"))]
    {
        new_backend.initialize(config)?;
    }

    #[cfg(feature = "tracing")]
    tracing::info!("Successfully switched to backend '{}'", new_name);

    Ok(())
}

// ============================================================================
// Async Backend Trait (feature-gated)
// ============================================================================

/// Async CAN hardware backend interface.
///
/// This trait provides asynchronous versions of the core message operations.
/// It is only available when the `async` feature is enabled.
///
/// # Feature Flags
///
/// - `async` - Enable async trait (requires runtime selection)
/// - `async-tokio` - Use tokio runtime
/// - `async-async-std` - Use async-std runtime
///
/// # Thread Safety
///
/// Like [`CanBackend`], this trait requires external synchronization.
/// Use `Arc<Mutex<>>` or `Arc<RwLock<>>` for shared access across tasks.
///
/// # Examples
///
/// ```rust,ignore
/// use canlink_hal::{CanBackendAsync, CanMessage};
///
/// async fn send_messages(backend: &mut impl CanBackendAsync) -> Result<(), Box<dyn std::error::Error>> {
///     let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4])?;
///     backend.send_message_async(&msg).await?;
///
///     if let Some(received) = backend.receive_message_async(Some(Duration::from_secs(1))).await? {
///         println!("Received: {:?}", received);
///     }
///     Ok(())
/// }
/// ```
#[cfg(feature = "async")]
#[allow(async_fn_in_trait)]
pub trait CanBackendAsync: CanBackend {
    /// Send a CAN message asynchronously.
    ///
    /// This is the async version of [`CanBackend::send_message`].
    ///
    /// # Arguments
    ///
    /// * `message` - The CAN message to send
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Message sent successfully
    /// * `Err(CanError)` - Send failed
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let msg = CanMessage::new_standard(0x123, &[0x01, 0x02])?;
    /// backend.send_message_async(&msg).await?;
    /// ```
    async fn send_message_async(&mut self, message: &CanMessage) -> CanResult<()>;

    /// Receive a CAN message asynchronously with optional timeout.
    ///
    /// This is the async version of [`CanBackend::receive_message`].
    /// Unlike the sync version, this method can wait for a message with a timeout.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Optional timeout duration. If `None`, returns immediately
    ///   like the sync version. If `Some(duration)`, waits up to that duration
    ///   for a message.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(message))` - A message was received
    /// * `Ok(None)` - No message available (timeout expired or no timeout and queue empty)
    /// * `Err(CanError)` - Reception failed
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Non-blocking receive
    /// if let Some(msg) = backend.receive_message_async(None).await? {
    ///     println!("Received: {:?}", msg);
    /// }
    ///
    /// // Receive with 1 second timeout
    /// match backend.receive_message_async(Some(Duration::from_secs(1))).await? {
    ///     Some(msg) => println!("Received: {:?}", msg),
    ///     None => println!("Timeout - no message received"),
    /// }
    /// ```
    async fn receive_message_async(
        &mut self,
        timeout: Option<Duration>,
    ) -> CanResult<Option<CanMessage>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock backend for testing
    struct TestBackend {
        initialized: bool,
        fail_count: u32,
    }

    impl TestBackend {
        fn new(fail_count: u32) -> Self {
            Self {
                initialized: false,
                fail_count,
            }
        }
    }

    impl CanBackend for TestBackend {
        fn initialize(&mut self, _config: &BackendConfig) -> CanResult<()> {
            if self.fail_count > 0 {
                self.fail_count -= 1;
                Err(CanError::InitializationFailed {
                    reason: "Test failure".to_string(),
                })
            } else {
                self.initialized = true;
                Ok(())
            }
        }

        fn close(&mut self) -> CanResult<()> {
            Ok(())
        }

        fn get_capability(&self) -> CanResult<HardwareCapability> {
            unimplemented!()
        }

        fn send_message(&mut self, _message: &CanMessage) -> CanResult<()> {
            unimplemented!()
        }

        fn receive_message(&mut self) -> CanResult<Option<CanMessage>> {
            unimplemented!()
        }

        fn open_channel(&mut self, _channel: u8) -> CanResult<()> {
            unimplemented!()
        }

        fn close_channel(&mut self, _channel: u8) -> CanResult<()> {
            unimplemented!()
        }

        fn version(&self) -> BackendVersion {
            BackendVersion::new(0, 1, 0)
        }

        fn name(&self) -> &'static str {
            "test"
        }
    }

    #[test]
    fn test_retry_initialize_success_first_attempt() {
        let mut backend = TestBackend::new(0);
        let config = BackendConfig::new("test");

        let result = retry_initialize(&mut backend, &config, 3, 10);
        assert!(result.is_ok());
        assert!(backend.initialized);
    }

    #[test]
    fn test_retry_initialize_success_after_retries() {
        let mut backend = TestBackend::new(2); // Fail 2 times, succeed on 3rd
        let config = BackendConfig::new("test");

        let result = retry_initialize(&mut backend, &config, 3, 10);
        assert!(result.is_ok());
        assert!(backend.initialized);
    }

    #[test]
    fn test_retry_initialize_failure_all_attempts() {
        let mut backend = TestBackend::new(10); // Fail all attempts
        let config = BackendConfig::new("test");

        let result = retry_initialize(&mut backend, &config, 3, 10);
        assert!(result.is_err());
        assert!(!backend.initialized);

        if let Err(CanError::InitializationFailed { reason }) = result {
            assert!(reason.contains("Failed after 4 attempts"));
        } else {
            panic!("Expected InitializationFailed error");
        }
    }
}

// ============================================================================
// High-Frequency Message Rate Monitor (FR-016)
// ============================================================================

/// Message rate monitor for detecting high-frequency message scenarios.
///
/// This utility helps backends detect when message rates exceed a threshold
/// and log warnings as specified in FR-016. It does not perform any automatic
/// throttling or backpressure - it only logs warnings for user awareness.
///
/// # Usage
///
/// Backends can use this to monitor receive rates:
///
/// ```rust,ignore
/// use canlink_hal::backend::MessageRateMonitor;
///
/// let mut monitor = MessageRateMonitor::new(1000); // Warn above 1000 msg/s
///
/// // In receive loop:
/// if let Some(msg) = backend.receive_message()? {
///     monitor.record_message();
///     process(msg);
/// }
/// ```
///
/// # Thread Safety
///
/// This struct is not thread-safe. Each thread should have its own monitor
/// or use external synchronization.
#[derive(Debug)]
pub struct MessageRateMonitor {
    /// Threshold in messages per second
    threshold_per_second: u32,
    /// Message count in current window
    message_count: u32,
    /// Start of current measurement window
    window_start: std::time::Instant,
    /// Whether we've already warned in this window
    warned_this_window: bool,
}

impl MessageRateMonitor {
    /// Create a new message rate monitor.
    ///
    /// # Arguments
    ///
    /// * `threshold_per_second` - Message rate threshold that triggers warnings
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::backend::MessageRateMonitor;
    ///
    /// // Warn when rate exceeds 1000 messages/second
    /// let monitor = MessageRateMonitor::new(1000);
    /// ```
    #[must_use]
    pub fn new(threshold_per_second: u32) -> Self {
        Self {
            threshold_per_second,
            message_count: 0,
            window_start: std::time::Instant::now(),
            warned_this_window: false,
        }
    }

    /// Record a message and check if rate exceeds threshold.
    ///
    /// Returns `true` if the rate exceeds the threshold (warning should be logged).
    /// Only returns `true` once per measurement window to avoid log spam.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::backend::MessageRateMonitor;
    ///
    /// let mut monitor = MessageRateMonitor::new(1000);
    ///
    /// // Record messages
    /// if monitor.record_message() {
    ///     // Rate exceeded threshold - handle warning
    ///     eprintln!("Warning: High message rate detected");
    /// }
    /// ```
    pub fn record_message(&mut self) -> bool {
        self.message_count += 1;

        let elapsed = self.window_start.elapsed();

        // Check every second
        if elapsed >= Duration::from_secs(1) {
            let exceeded = self.message_count > self.threshold_per_second;

            // Log warning if exceeded and haven't warned yet
            if exceeded && !self.warned_this_window {
                #[cfg(feature = "tracing")]
                tracing::warn!(
                    "High message rate detected: {} messages/second (threshold: {})",
                    self.message_count,
                    self.threshold_per_second
                );
                self.warned_this_window = true;
            }

            // Reset for next window
            self.message_count = 0;
            self.window_start = std::time::Instant::now();
            self.warned_this_window = false;

            exceeded
        } else {
            false
        }
    }

    /// Get the current message count in this window.
    #[must_use]
    pub fn current_count(&self) -> u32 {
        self.message_count
    }

    /// Get the configured threshold.
    #[must_use]
    pub fn threshold(&self) -> u32 {
        self.threshold_per_second
    }

    /// Reset the monitor state.
    pub fn reset(&mut self) {
        self.message_count = 0;
        self.window_start = std::time::Instant::now();
        self.warned_this_window = false;
    }
}

impl Default for MessageRateMonitor {
    /// Create a monitor with default threshold of 10000 messages/second.
    fn default() -> Self {
        Self::new(10000)
    }
}
