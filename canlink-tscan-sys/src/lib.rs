//! # canlink-tscan-sys
//!
//! Low-level FFI bindings to LibTSCAN for CAN hardware access.
//!
//! This crate provides unsafe, raw bindings to the LibTSCAN C API.
//! For a safe, high-level interface, use the `canlink-tscan` crate instead.
//!
//! ## Platform Support
//!
//! This crate currently only builds on Windows targets.
//! LibTSCAN documentation includes non-Windows artifacts, but this crate has not been verified or adapted there yet.
//!
//! ## Usage
//!
//! ```no_run
//! use canlink_tscan_sys::*;
//!
//! unsafe {
//!     // Initialize library
//!     initialize_lib_tscan(true, false, false);
//!
//!     // Scan for devices
//!     let mut device_count = 0;
//!     tscan_scan_devices(&mut device_count);
//!
//!     // Connect to default device
//!     let mut handle = 0;
//!     tscan_connect(std::ptr::null(), &mut handle);
//!
//!     // ... use the device ...
//!
//!     // Cleanup
//!     tscan_disconnect_by_handle(handle);
//!     finalize_lib_tscan();
//! }
//! ```
//!
//! ## Safety
//!
//! All functions in this crate are `unsafe` because they directly call C functions.
//! Callers must ensure:
//! - `initialize_lib_tscan()` is called before any other functions
//! - Device handles are valid
//! - Pointers are valid and properly aligned
//! - Buffers are large enough for the requested operations
//! - `finalize_lib_tscan()` is called when done
//!
//! ## Library Location
//!
//! The LibTSCAN.dll must be available in the system PATH or in the same directory
//! as the executable. Typically, it is located in the TSMaster installation
//! directory under `bin`, for example `TSMaster\\bin\\libTSCAN.dll`.

#![cfg(windows)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![deny(missing_docs)]

pub mod functions;
pub mod types;

#[cfg(test)]
mod bundle;

// Re-export everything for convenience
pub use functions::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(CHN1, 0);
        assert_eq!(CHN2, 1);
        assert_eq!(MASK_CANPROP_DIR_TX, 0x01);
        assert_eq!(MASK_CANPROP_EXTEND, 0x04);
    }

    #[test]
    fn test_enum_values() {
        assert_eq!(TLIBCANFDControllerType::lfdtCAN as i32, 0);
        assert_eq!(TLIBCANFDControllerType::lfdtISOCAN as i32, 1);
        assert_eq!(TLIBCANFDControllerMode::lfdmNormal as i32, 0);
    }
}
