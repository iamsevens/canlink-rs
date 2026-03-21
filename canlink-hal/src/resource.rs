//! Resource Management Guidelines and Best Practices (FR-012)
//!
//! This module provides documentation and utilities for proper resource management
//! in CANLink-RS. Following these guidelines ensures no memory leaks or handle leaks
//! in long-running applications.
//!
//! # Resource Management Philosophy
//!
//! CANLink-RS follows Rust's RAII (Resource Acquisition Is Initialization) pattern.
//! All resources are automatically released when their owning objects go out of scope.
//! This is achieved through proper implementation of the [`Drop`] trait.
//!
//! # Key Principles
//!
//! 1. **Automatic Cleanup**: All backend resources are released when the backend is dropped
//! 2. **Explicit Close**: Call `close()` for graceful shutdown with error handling
//! 3. **No Runtime Detection**: Resource leak detection is done through testing (valgrind/miri)
//! 4. **Drop Safety**: All types implement `Drop` to ensure cleanup even on panic
//!
//! # Resource Types
//!
//! ## Backend Resources
//!
//! [`CanBackend`](crate::CanBackend) implementations manage:
//! - Hardware handles (device connections)
//! - Internal message queues
//! - Channel state
//! - Filter configurations
//!
//! ```rust,ignore
//! use canlink_hal::{CanBackend, BackendConfig};
//!
//! fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut backend = create_backend();
//!     backend.initialize(&config)?;
//!     backend.open_channel(0)?;
//!
//!     // Use the backend...
//!
//!     // Option 1: Explicit close (recommended for error handling)
//!     backend.close()?;
//!
//!     // Option 2: Automatic cleanup on drop (always happens)
//!     // drop(backend); // implicit when going out of scope
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Queue Resources
//!
//! [`BoundedQueue`](crate::queue::BoundedQueue) manages:
//! - Message buffer memory
//! - Queue statistics
//!
//! Queues automatically release memory when dropped. No explicit cleanup needed.
//!
//! ## Filter Resources
//!
//! [`FilterChain`](crate::filter::FilterChain) manages:
//! - Filter objects (boxed trait objects)
//! - Hardware filter registrations
//!
//! Filters are automatically cleaned up when the chain is dropped.
//!
//! ## Monitor Resources
//!
//! [`ConnectionMonitor`](crate::monitor::ConnectionMonitor) manages:
//! - Monitoring thread/task
//! - Callback registrations
//!
//! Always call `stop()` before dropping to ensure clean shutdown:
//!
//! ```rust,ignore
//! use canlink_hal::monitor::ConnectionMonitor;
//!
//! fn example() {
//!     let mut monitor = ConnectionMonitor::new(backend, Duration::from_secs(1));
//!     monitor.start();
//!
//!     // Use the monitor...
//!
//!     monitor.stop(); // Recommended: explicit stop
//!     // drop(monitor); // Also works, but stop() is cleaner
//! }
//! ```
//!
//! ## Hot Reload Resources
//!
//! `ConfigWatcher` (when `hot-reload` feature enabled):
//! - File system watcher thread
//! - Callback registrations
//!
//! Always call `stop()` before dropping:
//!
//! ```rust,ignore
//! #[cfg(feature = "hot-reload")]
//! use canlink_hal::hot_reload::ConfigWatcher;
//!
//! #[cfg(feature = "hot-reload")]
//! fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut watcher = ConfigWatcher::new("config.toml")?;
//!     watcher.start()?;
//!
//!     // Use the watcher...
//!
//!     watcher.stop(); // Clean shutdown
//!     Ok(())
//! }
//! ```
//!
//! # Best Practices
//!
//! ## 1. Use RAII Patterns
//!
//! Let Rust's ownership system manage resources:
//!
//! ```rust,ignore
//! fn process_messages() -> Result<(), CanError> {
//!     let mut backend = create_backend();
//!     backend.initialize(&config)?;
//!
//!     // Backend is automatically closed when function returns
//!     // (either normally or via early return/panic)
//!     process(&mut backend)?;
//!
//!     Ok(())
//! } // backend.drop() called here
//! ```
//!
//! ## 2. Explicit Close for Error Handling
//!
//! When you need to handle close errors:
//!
//! ```rust,ignore
//! fn graceful_shutdown(backend: &mut dyn CanBackend) -> Result<(), CanError> {
//!     // Drain remaining messages first
//!     while let Some(msg) = backend.receive_message()? {
//!         process_message(msg);
//!     }
//!
//!     // Explicit close with error handling
//!     backend.close().map_err(|e| {
//!         eprintln!("Warning: close failed: {}", e);
//!         e
//!     })
//! }
//! ```
//!
//! ## 3. Scope-Based Resource Management
//!
//! Use scopes to control resource lifetime:
//!
//! ```rust,ignore
//! fn multi_backend_operation() -> Result<(), CanError> {
//!     // First backend scope
//!     {
//!         let mut backend1 = create_backend();
//!         backend1.initialize(&config1)?;
//!         use_backend(&mut backend1)?;
//!     } // backend1 released here
//!
//!     // Second backend scope (resources from backend1 are freed)
//!     {
//!         let mut backend2 = create_backend();
//!         backend2.initialize(&config2)?;
//!         use_backend(&mut backend2)?;
//!     } // backend2 released here
//!
//!     Ok(())
//! }
//! ```
//!
//! ## 4. Backend Switching
//!
//! Use [`switch_backend`](crate::switch_backend) for clean transitions:
//!
//! ```rust,ignore
//! use canlink_hal::switch_backend;
//!
//! fn switch_to_new_hardware(
//!     old: &mut dyn CanBackend,
//!     new: &mut dyn CanBackend,
//!     config: &BackendConfig,
//! ) -> Result<(), CanError> {
//!     // Process remaining messages before switch
//!     while let Some(msg) = old.receive_message()? {
//!         process_message(msg);
//!     }
//!
//!     // Clean switch (old backend closed, new initialized)
//!     switch_backend(old, new, config)
//! }
//! ```
//!
//! # Testing for Resource Leaks
//!
//! ## Using Valgrind (Linux)
//!
//! ```bash
//! cargo build --release
//! valgrind --leak-check=full ./target/release/your_app
//! ```
//!
//! ## Using Miri (Cross-platform)
//!
//! ```bash
//! cargo +nightly miri test
//! ```
//!
//! ## Integration Test Pattern
//!
//! ```rust,ignore
//! #[test]
//! fn test_no_resource_leak() {
//!     for _ in 0..1000 {
//!         let mut backend = MockBackend::new();
//!         backend.initialize(&config).unwrap();
//!         backend.open_channel(0).unwrap();
//!
//!         // Simulate usage
//!         for i in 0..100 {
//!             let msg = CanMessage::new_standard(i, &[0u8; 8]).unwrap();
//!             backend.send_message(&msg).unwrap();
//!         }
//!
//!         backend.close().unwrap();
//!     }
//!     // If this completes without OOM, no significant leaks
//! }
//! ```
//!
//! # Drop Trait Implementations
//!
//! All resource-holding types in CANLink-RS implement `Drop`:
//!
//! | Type | Drop Behavior |
//! |------|---------------|
//! | `MockBackend` | Closes channels, clears queues |
//! | `BoundedQueue` | Releases message buffer |
//! | `FilterChain` | Drops all filter objects |
//! | `ConnectionMonitor` | Stops monitoring, releases callbacks |
//! | `ConfigWatcher` | Stops file watcher thread |
//!
//! # Common Pitfalls
//!
//! ## 1. Forgetting to Stop Monitors
//!
//! ```rust,ignore
//! // BAD: Monitor thread may continue running
//! fn bad_example() {
//!     let mut monitor = ConnectionMonitor::new(backend, interval);
//!     monitor.start();
//!     // Oops, forgot to stop!
//! }
//!
//! // GOOD: Explicit stop
//! fn good_example() {
//!     let mut monitor = ConnectionMonitor::new(backend, interval);
//!     monitor.start();
//!     // ... use monitor ...
//!     monitor.stop();
//! }
//! ```
//!
//! ## 2. Holding References Across Backend Operations
//!
//! ```rust,ignore
//! // BAD: Holding reference while calling mutable method
//! fn bad_example(backend: &mut dyn CanBackend) {
//!     let name = backend.name(); // borrows backend
//!     backend.close(); // ERROR: cannot borrow mutably
//!     println!("Closed {}", name);
//! }
//!
//! // GOOD: Clone the name first
//! fn good_example(backend: &mut dyn CanBackend) {
//!     let name = backend.name().to_string(); // owned copy
//!     backend.close()?;
//!     println!("Closed {}", name);
//! }
//! ```
//!
//! ## 3. Circular References
//!
//! Avoid creating circular references with callbacks:
//!
//! ```rust,ignore
//! // BAD: Potential circular reference
//! let backend = Arc::new(Mutex::new(backend));
//! let backend_clone = Arc::clone(&backend);
//! monitor.on_state_change(move |state| {
//!     // This closure holds Arc<Mutex<Backend>>
//!     // If monitor is stored in backend, circular reference!
//! });
//!
//! // GOOD: Use weak references or restructure
//! let backend = Arc::new(Mutex::new(backend));
//! let backend_weak = Arc::downgrade(&backend);
//! monitor.on_state_change(move |state| {
//!     if let Some(backend) = backend_weak.upgrade() {
//!         // Safe: weak reference doesn't prevent drop
//!     }
//! });
//! ```

// This module is documentation-only. No runtime code.
// Resource management is enforced through Rust's type system and Drop trait.
