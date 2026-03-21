//! Low-level FFI type definitions for LibTSCAN
//!
//! This module contains the raw C types and structures used by LibTSCAN.

/// Unsigned 8-bit integer used by LibTSCAN.
pub type u8 = ::std::os::raw::c_uchar;
/// Signed 8-bit integer used by LibTSCAN.
pub type s8 = ::std::os::raw::c_char;
/// Unsigned 16-bit integer used by LibTSCAN.
pub type u16 = ::std::os::raw::c_ushort;
/// Signed 16-bit integer used by LibTSCAN.
pub type s16 = ::std::os::raw::c_short;
/// Unsigned 32-bit integer used by LibTSCAN.
pub type u32 = ::std::os::raw::c_uint;
/// Signed 32-bit integer used by LibTSCAN.
pub type s32 = ::std::os::raw::c_int;
/// Unsigned 64-bit integer used by LibTSCAN.
pub type u64 = ::std::os::raw::c_ulonglong;
/// Signed 64-bit integer used by LibTSCAN.
pub type s64 = ::std::os::raw::c_longlong;

// Channel definitions
/// Channel 1 index.
pub const CHN1: u32 = 0;
/// Channel 2 index.
pub const CHN2: u32 = 1;
/// Channel 3 index.
pub const CHN3: u32 = 2;
/// Channel 4 index.
pub const CHN4: u32 = 3;
/// Channel 5 index.
pub const CHN5: u32 = 4;
/// Channel 6 index.
pub const CHN6: u32 = 5;
/// Channel 7 index.
pub const CHN7: u32 = 6;
/// Channel 8 index.
pub const CHN8: u32 = 7;

// CAN message property bit masks
/// Marks a transmit frame.
pub const MASK_CANPROP_DIR_TX: u8 = 0x01;
/// Marks a remote frame.
pub const MASK_CANPROP_REMOTE: u8 = 0x02;
/// Marks an extended (29-bit) ID frame.
pub const MASK_CANPROP_EXTEND: u8 = 0x04;
/// Marks an error frame.
pub const MASK_CANPROP_ERROR: u8 = 0x80;

// CAN FD message property bit masks
/// Marks a CAN FD frame.
pub const MASK_CANFDPROP_IS_FD: u8 = 0x01;
/// Marks EDL in a CAN FD frame.
pub const MASK_CANFDPROP_IS_EDL: u8 = 0x01;
/// Marks BRS in a CAN FD frame.
pub const MASK_CANFDPROP_IS_BRS: u8 = 0x02;
/// Marks ESI in a CAN FD frame.
pub const MASK_CANFDPROP_IS_ESI: u8 = 0x04;

// RX/TX filter
/// Receive-only filter mode.
pub const ONLY_RX_MESSAGES: u8 = 0;
/// Receive and transmit filter mode.
pub const TX_RX_MESSAGES: u8 = 1;

/// CAN FD Controller Type
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TLIBCANFDControllerType {
    /// CAN 2.0
    lfdtCAN = 0,
    /// ISO CAN-FD
    lfdtISOCAN = 1,
    /// Non-ISO CAN-FD
    lfdtNonISOCAN = 2,
}

/// CAN FD Controller Mode
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TLIBCANFDControllerMode {
    /// Normal mode
    lfdmNormal = 0,
    /// ACK off (listen-only)
    lfdmACKOff = 1,
    /// Restricted mode
    lfdmRestricted = 2,
}

/// CAN message structure (16 bytes, packed)
#[repr(C, packed)]
#[derive(Debug, Copy, Clone, Default)]
pub struct TLIBCAN {
    /// Channel index (0-based)
    pub FIdxChn: u8,
    /// Properties: bit7=error, bit2=extended, bit1=remote, bit0=tx
    pub FProperties: u8,
    /// Data length code (0-8)
    pub FDLC: u8,
    /// Reserved for alignment
    pub FReserved: u8,
    /// CAN identifier
    pub FIdentifier: s32,
    /// Timestamp in microseconds
    pub FTimeUs: s64,
    /// Data bytes (up to 8)
    pub FData: [u8; 8],
}

/// CAN-FD message structure (80 bytes, packed)
#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct TLIBCANFD {
    /// Channel index (0-based)
    pub FIdxChn: u8,
    /// Properties: bit7=error, bit2=extended, bit1=remote, bit0=tx
    pub FProperties: u8,
    /// Data length code (0-15)
    pub FDLC: u8,
    /// FD Properties: bit0=EDL, bit1=BRS, bit2=ESI
    pub FFDProperties: u8,
    /// CAN identifier
    pub FIdentifier: s32,
    /// Timestamp in microseconds
    pub FTimeUs: s64,
    /// Data bytes (up to 64)
    pub FData: [u8; 64],
}

impl Default for TLIBCANFD {
    fn default() -> Self {
        Self {
            FIdxChn: 0,
            FProperties: 0,
            FDLC: 0,
            FFDProperties: 0,
            FIdentifier: 0,
            FTimeUs: 0,
            FData: [0; 64],
        }
    }
}

// Callback function types
/// Callback type for CAN queue events on Windows.
pub type TCANQueueEvent_Win32 = Option<unsafe extern "system" fn(AData: *const TLIBCAN)>;
/// Callback type for CAN FD queue events on Windows.
pub type TCANFDQueueEvent_Win32 = Option<unsafe extern "system" fn(AData: *const TLIBCANFD)>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn test_tlibcan_size() {
        // TLIBCAN should be 24 bytes (packed)
        assert_eq!(mem::size_of::<TLIBCAN>(), 24);
    }

    #[test]
    fn test_tlibcanfd_size() {
        // TLIBCANFD should be 80 bytes (packed)
        assert_eq!(mem::size_of::<TLIBCANFD>(), 80);
    }

    #[test]
    fn test_default_message() {
        let msg = TLIBCAN::default();
        // Copy fields to avoid unaligned references
        let idx_chn = msg.FIdxChn;
        let properties = msg.FProperties;
        let dlc = msg.FDLC;
        let identifier = msg.FIdentifier;

        assert_eq!(idx_chn, 0);
        assert_eq!(properties, 0);
        assert_eq!(dlc, 0);
        assert_eq!(identifier, 0);
    }
}
