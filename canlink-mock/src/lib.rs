//! # `CANLink` Mock Backend
//!
//! This crate provides a mock implementation of the CAN hardware abstraction layer
//! for testing purposes. It allows testing CAN applications without physical hardware.
//!
//! ## Features
//!
//! - **Message Recording**: Records all sent messages for verification
//! - **Preset Messages**: Configure messages to be received
//! - **Error Injection**: Simulate hardware errors and failures
//! - **Behavior Verification**: Validate application behavior in tests
//! - **Configurable Capabilities**: Simulate different hardware configurations
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use canlink_mock::MockBackend;
//! use canlink_hal::{BackendConfig, CanBackend, CanMessage, CanId};
//!
//! // Create and initialize backend
//! let mut backend = MockBackend::new();
//! let config = BackendConfig::new("mock");
//! backend.initialize(&config)?;
//! backend.open_channel(0)?;
//!
//! // Send a message
//! let msg = CanMessage::new_standard(0x123, &[1, 2, 3])?;
//! backend.send_message(&msg)?;
//!
//! // Verify it was recorded
//! assert!(backend.verify_message_sent(CanId::Standard(0x123)));
//! let recorded = backend.get_recorded_messages();
//! assert_eq!(recorded.len(), 1);
//! ```
//!
//! ## Message Recording
//!
//! The mock backend automatically records all sent messages:
//!
//! ```rust,ignore
//! use canlink_mock::MockBackend;
//! use canlink_hal::{CanBackend, CanMessage, CanId};
//!
//! let mut backend = MockBackend::new();
//! backend.initialize(&config)?;
//! backend.open_channel(0)?;
//!
//! // Send multiple messages
//! backend.send_message(&CanMessage::new_standard(0x100, &[1, 2])?)?;
//! backend.send_message(&CanMessage::new_standard(0x200, &[3, 4])?)?;
//! backend.send_message(&CanMessage::new_standard(0x100, &[5, 6])?)?;
//!
//! // Verify messages
//! assert!(backend.verify_message_count(3));
//! assert!(backend.verify_message_sent(CanId::Standard(0x100)));
//!
//! // Get messages by ID
//! let messages = backend.get_messages_by_id(CanId::Standard(0x100));
//! assert_eq!(messages.len(), 2);
//! assert_eq!(messages[0].data(), &[1, 2]);
//! assert_eq!(messages[1].data(), &[5, 6]);
//! ```
//!
//! ## Preset Messages
//!
//! Configure messages to be returned by `receive_message()`:
//!
//! ```rust,ignore
//! use canlink_mock::{MockBackend, MockConfig};
//! use canlink_hal::{CanBackend, CanMessage};
//!
//! // Create preset messages
//! let preset = vec![
//!     CanMessage::new_standard(0x111, &[0x11, 0x22])?,
//!     CanMessage::new_standard(0x222, &[0x33, 0x44])?,
//! ];
//!
//! // Create backend with preset messages
//! let config = MockConfig::with_preset_messages(preset);
//! let mut backend = MockBackend::with_config(config);
//! backend.initialize(&backend_config)?;
//! backend.open_channel(0)?;
//!
//! // Receive preset messages
//! let msg1 = backend.receive_message()?.unwrap();
//! assert_eq!(msg1.id(), CanId::Standard(0x111));
//!
//! let msg2 = backend.receive_message()?.unwrap();
//! assert_eq!(msg2.id(), CanId::Standard(0x222));
//!
//! // No more messages
//! assert!(backend.receive_message()?.is_none());
//! ```
//!
//! ## Error Injection
//!
//! Simulate hardware errors for testing error handling:
//!
//! ```rust,ignore
//! use canlink_mock::MockBackend;
//! use canlink_hal::{CanBackend, CanError, CanMessage};
//!
//! let mut backend = MockBackend::new();
//! backend.initialize(&config)?;
//! backend.open_channel(0)?;
//!
//! // Inject a send error
//! backend.error_injector_mut().inject_send_error(
//!     CanError::SendFailed {
//!         reason: "Bus-Off state".to_string(),
//!     }
//! );
//!
//! // Next send will fail
//! let msg = CanMessage::new_standard(0x123, &[1, 2, 3])?;
//! let result = backend.send_message(&msg);
//! assert!(result.is_err());
//!
//! // Failed messages are not recorded
//! assert_eq!(backend.get_recorded_messages().len(), 0);
//! ```
//!
//! ### Advanced Error Injection
//!
//! Control when errors occur with skip and count parameters:
//!
//! ```rust,ignore
//! // Fail the 3rd and 4th send attempts
//! backend.error_injector_mut().inject_send_error_with_config(
//!     CanError::SendFailed { reason: "Test".to_string() },
//!     2,  // inject 2 times
//!     2,  // skip first 2 calls
//! );
//!
//! // First two sends succeed
//! backend.send_message(&msg1)?; // OK
//! backend.send_message(&msg2)?; // OK
//!
//! // Next two fail
//! assert!(backend.send_message(&msg3).is_err());
//! assert!(backend.send_message(&msg4).is_err());
//!
//! // Fifth succeeds (injection exhausted)
//! backend.send_message(&msg5)?; // OK
//! ```
//!
//! ## Configuration
//!
//! Customize the mock backend behavior:
//!
//! ```rust,ignore
//! use canlink_mock::{MockBackend, MockConfig};
//!
//! // CAN 2.0 only (no CAN-FD)
//! let config = MockConfig::can20_only();
//! let backend = MockBackend::with_config(config);
//!
//! // Custom configuration
//! let mut config = MockConfig::new();
//! config.channel_count = 4;
//! config.supports_canfd = true;
//! config.max_bitrate = 2_000_000;
//! config.supported_bitrates = vec![125_000, 250_000, 500_000, 1_000_000];
//! let backend = MockBackend::with_config(config);
//! ```
//!
//! ## Testing Patterns
//!
//! ### Protocol Testing
//!
//! Test request-response protocols:
//!
//! ```rust,ignore
//! // Setup preset responses
//! let responses = vec![
//!     CanMessage::new_standard(0x7E8, &[0x04, 0x41, 0x0C, 0x1A, 0xF8])?,
//! ];
//! let config = MockConfig::with_preset_messages(responses);
//! let mut backend = MockBackend::with_config(config);
//!
//! // Send request
//! let request = CanMessage::new_standard(0x7DF, &[0x02, 0x01, 0x0C])?;
//! backend.send_message(&request)?;
//!
//! // Receive response
//! let response = backend.receive_message()?.unwrap();
//! assert_eq!(response.id(), CanId::Standard(0x7E8));
//!
//! // Verify request was sent
//! assert!(backend.verify_message_sent(CanId::Standard(0x7DF)));
//! ```
//!
//! ### Error Recovery Testing
//!
//! Test retry logic:
//!
//! ```rust,ignore
//! // Fail first 3 attempts
//! backend.error_injector_mut().inject_send_error_with_config(
//!     CanError::SendFailed { reason: "Busy".to_string() },
//!     3,  // fail 3 times
//!     0,  // no skip
//! );
//!
//! // Implement retry logic
//! let msg = CanMessage::new_standard(0x123, &[1, 2, 3])?;
//! let mut attempts = 0;
//! let max_retries = 5;
//!
//! while attempts < max_retries {
//!     match backend.send_message(&msg) {
//!         Ok(_) => break,
//!         Err(_) => attempts += 1,
//!     }
//! }
//!
//! assert!(attempts < max_retries);
//! assert_eq!(backend.get_recorded_messages().len(), 1);
//! ```
//!
//! ## Examples
//!
//! See the `examples/` directory for complete examples:
//!
//! - `mock_testing.rs` - Comprehensive mock testing demonstration
//! - `automated_testing.rs` - Automated test suite using mock backend
//!
//! ## See Also
//!
//! - [`canlink-hal`](https://docs.rs/canlink-hal) - Hardware abstraction layer
//! - [`canlink-cli`](https://docs.rs/canlink-cli) - Command-line interface
//!

#![deny(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

// Core modules
pub mod backend;
pub mod config;
pub mod injector;
pub mod recorder;

// Re-exports
pub use backend::{MockBackend, MockBackendFactory};
pub use config::MockConfig;
pub use injector::{ErrorInjector, ErrorType};
pub use recorder::MessageRecorder;
