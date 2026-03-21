//! # `CANLink` Hardware Abstraction Layer
//!
//! This crate provides a unified hardware abstraction layer for CAN bus interfaces.
//! It allows applications to work with different CAN hardware backends through a
//! common trait-based interface.
//!
//! ## Features
//!
//! - **Unified Interface**: Single trait ([`CanBackend`]) for all hardware backends
//! - **Backend Registry**: Runtime registration and discovery of hardware backends
//! - **Hardware Capabilities**: Query hardware features at runtime
//! - **Zero-Cost Abstraction**: Minimal performance overhead (< 5%)
//! - **Type Safety**: Compile-time guarantees through Rust's type system
//! - **Async Support**: Optional async API through feature flags
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use canlink_hal::{BackendConfig, CanBackend, CanMessage, CanId};
//! use canlink_mock::MockBackend;
//!
//! // Create and initialize backend
//! let mut backend = MockBackend::new();
//! let config = BackendConfig::new("mock");
//! backend.initialize(&config)?;
//!
//! // Open a channel
//! backend.open_channel(0)?;
//!
//! // Send a message
//! let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4])?;
//! backend.send_message(&msg)?;
//!
//! // Receive a message
//! if let Some(msg) = backend.receive_message()? {
//!     println!("Received: ID={:?}, Data={:?}", msg.id(), msg.data());
//! }
//!
//! // Clean up
//! backend.close_channel(0)?;
//! backend.close()?;
//! ```
//!
//! ## Architecture
//!
//! The abstraction layer consists of several key components:
//!
//! ### Core Traits
//!
//! - **[`CanBackend`]**: Core interface that all backends must implement
//!   - `initialize()` - Initialize the backend
//!   - `send_message()` - Send a CAN message
//!   - `receive_message()` - Receive a CAN message
//!   - `open_channel()` / `close_channel()` - Channel management
//!   - `get_capability()` - Query hardware capabilities
//!
//! - **[`BackendFactory`]**: Factory trait for creating backend instances
//!
//! ### Key Types
//!
//! - **[`CanMessage`]**: Unified CAN message representation
//!   - Supports standard (11-bit) and extended (29-bit) IDs
//!   - Supports CAN 2.0 and CAN-FD frames
//!   - Supports remote frames
//!
//! - **[`CanId`]**: CAN identifier (standard or extended)
//!
//! - **[`HardwareCapability`]**: Hardware feature description
//!   - Channel count
//!   - CAN-FD support
//!   - Supported bitrates
//!   - Hardware filter count
//!   - Timestamp precision
//!
//! - **[`BackendRegistry`]**: Manages backend registration and instantiation
//!
//! - **[`CanError`]**: Common error types across all backends
//!
//! ## Usage Examples
//!
//! ### Basic Message Sending
//!
//! ```rust,ignore
//! use canlink_hal::{CanBackend, CanMessage};
//! use canlink_mock::MockBackend;
//!
//! let mut backend = MockBackend::new();
//! backend.initialize(&config)?;
//! backend.open_channel(0)?;
//!
//! // Standard CAN message
//! let msg = CanMessage::new_standard(0x123, &[0xAA, 0xBB, 0xCC, 0xDD])?;
//! backend.send_message(&msg)?;
//!
//! // Extended ID message
//! let msg = CanMessage::new_extended(0x12345678, &[1, 2, 3, 4, 5, 6, 7, 8])?;
//! backend.send_message(&msg)?;
//!
//! // CAN-FD message (if supported)
//! let data = vec![0; 64]; // Up to 64 bytes
//! let msg = CanMessage::new_fd(CanId::Standard(0x200), &data)?;
//! backend.send_message(&msg)?;
//! ```
//!
//! ### Capability-Based Adaptation
//!
//! ```rust,ignore
//! use canlink_hal::{CanBackend, CanMessage, CanId};
//!
//! let capability = backend.get_capability()?;
//!
//! // Adapt message type based on hardware support
//! let data = vec![0; 12];
//! let msg = if capability.supports_canfd {
//!     CanMessage::new_fd(CanId::Standard(0x123), &data)?
//! } else {
//!     // Split into multiple CAN 2.0 messages
//!     CanMessage::new_standard(0x123, &data[..8])?
//! };
//!
//! // Check bitrate support
//! if capability.supports_bitrate(1_000_000) {
//!     println!("1 Mbps is supported");
//! }
//!
//! // Check channel availability
//! if capability.has_channel(2) {
//!     backend.open_channel(2)?;
//! }
//! ```
//!
//! ### Error Handling
//!
//! ```rust,ignore
//! use canlink_hal::{CanError, CanBackend};
//!
//! match backend.send_message(&msg) {
//!     Ok(_) => println!("Message sent successfully"),
//!     Err(CanError::SendFailed { reason }) => {
//!         eprintln!("Send failed: {}", reason);
//!     }
//!     Err(CanError::BusError { kind }) => {
//!         eprintln!("Bus error: {:?}", kind);
//!     }
//!     Err(e) => eprintln!("Other error: {}", e),
//! }
//! ```
//!
//! ### Backend Registry
//!
//! ```rust,ignore
//! use canlink_hal::{BackendRegistry, BackendConfig};
//! use canlink_mock::MockBackendFactory;
//! use std::sync::Arc;
//!
//! // Get global registry
//! let registry = BackendRegistry::global();
//!
//! // Register a backend
//! let factory = Arc::new(MockBackendFactory::new());
//! registry.register(factory)?;
//!
//! // List available backends
//! for name in registry.list_backends() {
//!     println!("Available backend: {}", name);
//! }
//!
//! // Create backend instance
//! let config = BackendConfig::new("mock");
//! let mut backend = registry.create("mock", &config)?;
//! ```
//!
//! ## Testing
//!
//! The crate includes a mock backend for testing without hardware:
//!
//! ```rust,ignore
//! use canlink_hal::{CanBackend, CanMessage, CanId};
//! use canlink_mock::MockBackend;
//!
//! #[test]
//! fn test_can_communication() {
//!     let mut backend = MockBackend::new();
//!     backend.initialize(&config).unwrap();
//!     backend.open_channel(0).unwrap();
//!
//!     // Send message
//!     let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
//!     backend.send_message(&msg).unwrap();
//!
//!     // Verify message was sent
//!     assert!(backend.verify_message_sent(CanId::Standard(0x123)));
//! }
//! ```
//!
//! ## Feature Flags
//!
//! - `async` - Enable async API support
//! - `async-tokio` - Use tokio runtime for async operations
//! - `async-async-std` - Use async-std runtime for async operations
//! - `tracing` - Enable logging support via tracing framework
//! - `hot-reload` - Enable configuration hot-reload support
//! - `full` - Enable all features
//!
//! ## Performance
//!
//! The abstraction layer is designed for minimal overhead:
//!
//! - Trait method calls are typically inlined
//! - Zero-copy message passing where possible
//! - Abstraction overhead < 5% compared to direct hardware access
//!
//! ## Thread Safety
//!
//! Backend implementations follow an external synchronization model:
//!
//! - Backends are `Send` but not `Sync`
//! - Each thread should have its own backend instance
//! - Use channels or locks for cross-thread communication
//!
//! ## Supported Backends
//!
//! - **Mock**: Software simulation for testing (included)
//! - **`SocketCAN`**: Linux CAN interface (planned)
//! - **PCAN**: PEAK-System CAN adapters (planned)
//! - **IXXAT**: HMS IXXAT CAN interfaces (planned)
//! - **Kvaser**: Kvaser CAN devices (planned)
//!
//! ## See Also
//!
//! - [`canlink-mock`](https://docs.rs/canlink-mock) - Mock backend for testing
//! - [`canlink-cli`](https://docs.rs/canlink-cli) - Command-line interface
//! - Examples: see the workspace `examples/` directory
//!

#![deny(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

// Core modules
pub mod backend;
pub mod capability;
pub mod config;
pub mod error;
pub mod message;
pub mod registry;
pub mod state;
pub mod version;

// New modules for v0.2.0 (003-async-and-filtering)
pub mod filter;
pub mod monitor;
pub mod queue;

// Conditional modules
#[cfg(feature = "tracing")]
pub mod logging;

#[cfg(feature = "hot-reload")]
pub mod hot_reload;

// Periodic message sending (004 spec FR-001 to FR-006)
#[cfg(feature = "periodic")]
pub mod periodic;

// ISO-TP protocol support (004 spec FR-007 to FR-019)
#[cfg(feature = "isotp")]
pub mod isotp;

// Resource management documentation
pub mod resource;

// Re-exports
#[cfg(feature = "async")]
pub use backend::CanBackendAsync;
pub use backend::{
    retry_initialize, switch_backend, BackendFactory, CanBackend, MessageRateMonitor,
};
pub use capability::{HardwareCapability, TimestampPrecision};
pub use config::{BackendConfig, CanlinkConfig};
pub use error::{BusErrorKind, CanError, CanResult};
pub use error::{FilterError, FilterResult};
pub use error::{MonitorError, MonitorResult};
pub use error::{QueueError, QueueResult};
pub use message::{CanId, CanMessage, MessageFlags, Timestamp};
pub use registry::{BackendInfo, BackendRegistry};
pub use state::BackendState;
pub use version::BackendVersion;

// Periodic message sending re-exports (004 spec)
#[cfg(feature = "periodic")]
pub use periodic::{
    run_scheduler, PeriodicMessage, PeriodicScheduler, PeriodicStats, SchedulerCommand,
};

// ISO-TP protocol re-exports (004 spec)
#[cfg(feature = "isotp")]
pub use isotp::{
    AddressingMode, FlowStatus, FrameSize, IsoTpConfig, IsoTpConfigBuilder, IsoTpError, IsoTpFrame,
    IsoTpState, RxState, StMin, TxState,
};
