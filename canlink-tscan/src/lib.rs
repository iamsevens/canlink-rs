//! Safe Rust wrapper for LibTSCAN-backed CAN hardware.
//!
//! This crate provides a high-level, safe interface to CAN hardware reachable
//! through the LibTSCAN library. It implements the `CanBackend` trait from
//! `canlink-hal`, providing a unified interface for CAN communication.
//!
//! # Scope
//!
//! `canlink-tscan` is currently the only landed real-hardware backend in
//! CANLink-RS.
//!
//! - Actual hardware regression in this repository is currently limited to
//!   TOSUN-related devices.
//! - TSMaster/LibTSCAN headers expose multiple device types on the same backend
//!   path, so other LibTSCAN-visible hardware may work through the same DLL
//!   route.
//! - Those additional device types are not yet individually validated or
//!   promised by this crate.
//! - CANLink-RS can host additional native backends in the future without going
//!   through LibTSCAN, but those are not implemented here.
//!
//! # Features
//!
//! - Safe Rust wrapper around LibTSCAN FFI bindings
//! - Implements `CanBackend` trait for hardware abstraction
//! - Support for CAN 2.0 and CAN-FD
//! - Automatic resource management (RAII)
//! - Thread-safe with external synchronization
//!
//! # Examples
//!
//! ```ignore
//! use canlink_tscan::TSCanBackend;
//! use canlink_hal::{CanBackend, BackendConfig, CanMessage};
//!
//! // Create and initialize backend
//! let mut backend = TSCanBackend::new();
//! backend.initialize(&BackendConfig::new("tscan"))?;
//!
//! // Open channel
//! backend.open_channel(0)?;
//!
//! // Send a message
//! let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4])?;
//! backend.send_message(&msg)?;
//!
//! // Receive messages
//! while let Some(msg) = backend.receive_message()? {
//!     println!("Received: {:?}", msg);
//! }
//!
//! // Clean up
//! backend.close()?;
//! ```
//!
//! # Platform Support
//!
//! LibTSCAN supports multiple platforms (Windows, Linux, macOS), but this crate
//! currently implements Windows support only.
//!
//! **Current Implementation Status**:
//! - ✅ Windows (x64): Fully supported
//! - ⏳ Linux: Planned for future release
//! - ⏳ macOS: Planned for future release
//!
//! **Requirements**:
//! - LibTSCAN-compatible hardware runtime
//! - Validated hardware in this repository is currently limited to TOSUN-related devices
//! - LibTSCAN library (libTSCAN.dll on Windows)
//! - Windows 10/11 x64 platform (current implementation)

#![deny(missing_docs)]

mod backend;
mod config;
mod convert;
#[doc(hidden)]
pub mod daemon;
mod error;

pub use backend::{TSCanBackend, TSCanBackendFactory};
pub use config::{FileConfig, TscanDaemonConfig};

// Re-export commonly used types from canlink-hal
pub use canlink_hal::{
    BackendConfig, BackendVersion, CanBackend, CanError, CanId, CanMessage, CanResult,
    HardwareCapability, MessageFlags, Timestamp,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_creation() {
        let backend = TSCanBackend::new();
        assert_eq!(backend.name(), "tscan");
    }
}
