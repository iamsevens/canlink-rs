//! Message conversion between `LibTSCAN` and HAL types.
//!
//! This module provides conversion functions between `LibTSCAN`'s native
//! message structures (TLIBCAN, TLIBCANFD) and the unified HAL types
//! (CanMessage, CanId, etc.).

use canlink_hal::{CanError, CanId, CanMessage, MessageFlags, Timestamp};
use canlink_tscan_sys::types::*;

/// Convert `LibTSCAN` TLIBCAN to HAL CanMessage.
///
/// # Arguments
/// * `msg` - `LibTSCAN` CAN message
///
/// # Returns
/// Converted HAL CanMessage
///
/// # Examples
///
/// ```ignore
/// let libcan_msg = TLIBCAN { ... };
/// let hal_msg = from_tlibcan(&libcan_msg);
/// ```
pub fn from_tlibcan(msg: &TLIBCAN) -> CanMessage {
    // Copy fields from packed struct to avoid unaligned references
    let identifier = msg.FIdentifier;
    let properties = msg.FProperties;
    let dlc = msg.FDLC;
    let time_us = msg.FTimeUs;
    let data = msg.FData;

    // Determine if extended or standard ID
    let is_extended = (properties & MASK_CANPROP_EXTEND) != 0;
    let id = if is_extended {
        CanId::Extended(identifier as u32)
    } else {
        CanId::Standard((identifier & 0x7FF) as u16)
    };

    // Extract data (up to DLC bytes)
    let data_len = (dlc as usize).min(8);
    let data_vec = data[..data_len].to_vec();

    // Check if remote frame
    let is_remote = (properties & MASK_CANPROP_REMOTE) != 0;

    // Create message
    let mut message = if is_remote {
        CanMessage::new_remote(id, dlc).unwrap()
    } else if is_extended {
        CanMessage::new_extended(id.raw(), &data_vec).unwrap()
    } else {
        CanMessage::new_standard(id.raw() as u16, &data_vec).unwrap()
    };

    // Set timestamp if available
    if time_us > 0 {
        message.set_timestamp(Timestamp::from_micros(time_us as u64));
    }

    message
}

/// Convert HAL CanMessage to `LibTSCAN` TLIBCAN.
///
/// # Arguments
/// * `msg` - HAL CAN message
/// * `channel` - Channel index (0-based)
///
/// # Returns
/// * `Ok(TLIBCAN)` - Converted `LibTSCAN` message
/// * `Err(CanError)` - If message cannot be converted (e.g., data too long)
///
/// # Examples
///
/// ```ignore
/// let hal_msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4])?;
/// let libcan_msg = to_tlibcan(&hal_msg, 0)?;
/// ```
pub fn to_tlibcan(msg: &CanMessage, channel: u8) -> Result<TLIBCAN, CanError> {
    // Validate data length for CAN 2.0
    if msg.data().len() > 8 {
        return Err(CanError::InvalidDataLength {
            expected: 8,
            actual: msg.data().len(),
        });
    }

    // Build properties byte
    let mut properties: u8 = 0;

    // Set TX bit (we're transmitting)
    properties |= MASK_CANPROP_DIR_TX;

    // Set extended bit if needed
    if msg.id().is_extended() {
        properties |= MASK_CANPROP_EXTEND;
    }

    // Set RTR bit if needed
    if msg.flags().contains(MessageFlags::RTR) {
        properties |= MASK_CANPROP_REMOTE;
    }

    // Get identifier value
    let identifier = msg.id().raw() as s32;

    // Copy data
    let mut data = [0u8; 8];
    let data_len = msg.data().len();
    data[..data_len].copy_from_slice(msg.data());

    // Build TLIBCAN structure
    Ok(TLIBCAN {
        FIdxChn: channel,
        FProperties: properties,
        FDLC: data_len as u8,
        FReserved: 0,
        FIdentifier: identifier,
        FTimeUs: msg.timestamp().map_or(0, |ts| ts.as_micros() as s64),
        FData: data,
    })
}

/// Convert `LibTSCAN` TLIBCANFD to HAL CanMessage.
///
/// # Arguments
/// * `msg` - `LibTSCAN` CAN-FD message
///
/// # Returns
/// Converted HAL CanMessage with FD flags
///
/// # Examples
///
/// ```ignore
/// let libcanfd_msg = TLIBCANFD { ... };
/// let hal_msg = from_tlibcanfd(&libcanfd_msg);
/// ```
pub fn from_tlibcanfd(msg: &TLIBCANFD) -> CanMessage {
    // Copy fields from packed struct to avoid unaligned references
    let identifier = msg.FIdentifier;
    let properties = msg.FProperties;
    let dlc = msg.FDLC;
    let time_us = msg.FTimeUs;
    let data = msg.FData;

    // Determine if extended or standard ID
    let is_extended = (properties & MASK_CANPROP_EXTEND) != 0;
    let id = if is_extended {
        CanId::Extended(identifier as u32)
    } else {
        CanId::Standard((identifier & 0x7FF) as u16)
    };

    // Extract data (up to DLC bytes, max 64 for CAN-FD)
    let data_len = dlc_to_len(dlc).min(64);
    let data_vec = data[..data_len].to_vec();

    // Check if remote frame
    let is_remote = (properties & MASK_CANPROP_REMOTE) != 0;

    // Create message (CAN-FD)
    let mut message = if is_remote {
        CanMessage::new_remote(id, dlc).unwrap()
    } else {
        CanMessage::new_fd(id, &data_vec).unwrap()
    };

    // Set timestamp if available
    if time_us > 0 {
        message.set_timestamp(Timestamp::from_micros(time_us as u64));
    }

    message
}

/// Convert HAL CanMessage to `LibTSCAN` TLIBCANFD.
///
/// # Arguments
/// * `msg` - HAL CAN message (should have FD flag set)
/// * `channel` - Channel index (0-based)
///
/// # Returns
/// * `Ok(TLIBCANFD)` - Converted `LibTSCAN` CAN-FD message
/// * `Err(CanError)` - If message cannot be converted (e.g., data too long)
///
/// # Examples
///
/// ```ignore
/// let hal_msg = CanMessage::new_fd(CanId::Standard(0x123), &[1; 64])?;
/// let libcanfd_msg = to_tlibcanfd(&hal_msg, 0)?;
/// ```
pub fn to_tlibcanfd(msg: &CanMessage, channel: u8) -> Result<TLIBCANFD, CanError> {
    // Validate data length for CAN-FD
    if msg.data().len() > 64 {
        return Err(CanError::InvalidDataLength {
            expected: 64,
            actual: msg.data().len(),
        });
    }

    // Build properties byte
    let mut properties: u8 = 0;

    // Set TX bit (we're transmitting)
    properties |= MASK_CANPROP_DIR_TX;

    // Set extended bit if needed
    if msg.id().is_extended() {
        properties |= MASK_CANPROP_EXTEND;
    }

    // Set RTR bit if needed
    if msg.flags().contains(MessageFlags::RTR) {
        properties |= MASK_CANPROP_REMOTE;
    }

    // Build FD properties byte
    let mut fd_properties: u8 = MASK_CANFDPROP_IS_FD; // Always set FD bit

    if msg.flags().contains(MessageFlags::BRS) {
        fd_properties |= MASK_CANFDPROP_IS_BRS;
    }
    if msg.flags().contains(MessageFlags::ESI) {
        fd_properties |= MASK_CANFDPROP_IS_ESI;
    }

    // Get identifier value
    let identifier = msg.id().raw() as s32;

    // Copy data
    let mut data = [0u8; 64];
    let data_len = msg.data().len();
    data[..data_len].copy_from_slice(msg.data());

    // Convert data length to DLC
    let dlc = len_to_dlc(data_len as u8);

    // Build TLIBCANFD structure
    Ok(TLIBCANFD {
        FIdxChn: channel,
        FProperties: properties,
        FDLC: dlc,
        FFDProperties: fd_properties,
        FIdentifier: identifier,
        FTimeUs: msg.timestamp().map_or(0, |ts| ts.as_micros() as s64),
        FData: data,
    })
}

/// Convert CAN-FD DLC code to actual data length.
///
/// CAN-FD uses special DLC encoding for lengths > 8:
/// - 0-8: Direct mapping
/// - 9: 12 bytes
/// - 10: 16 bytes
/// - 11: 20 bytes
/// - 12: 24 bytes
/// - 13: 32 bytes
/// - 14: 48 bytes
/// - 15: 64 bytes
fn dlc_to_len(dlc: u8) -> usize {
    match dlc {
        0..=8 => dlc as usize,
        9 => 12,
        10 => 16,
        11 => 20,
        12 => 24,
        13 => 32,
        14 => 48,
        15 => 64,
        _ => 0, // Invalid DLC
    }
}

/// Convert data length to CAN-FD DLC code.
///
/// Rounds up to the next valid CAN-FD length.
fn len_to_dlc(len: u8) -> u8 {
    match len {
        0..=8 => len,
        9..=12 => 9,
        13..=16 => 10,
        17..=20 => 11,
        21..=24 => 12,
        25..=32 => 13,
        33..=48 => 14,
        49..=64 => 15,
        _ => 15, // Cap at 64 bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_tlibcan_standard() {
        let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4]).unwrap();
        let libcan = to_tlibcan(&msg, 0).unwrap();

        // Copy fields to avoid unaligned references
        let idx_chn = libcan.FIdxChn;
        let identifier = libcan.FIdentifier;
        let dlc = libcan.FDLC;
        let data = libcan.FData;
        let properties = libcan.FProperties;

        assert_eq!(idx_chn, 0);
        assert_eq!(identifier, 0x123);
        assert_eq!(dlc, 4);
        assert_eq!(&data[..4], &[1, 2, 3, 4]);
        assert_eq!(properties & MASK_CANPROP_EXTEND, 0);
        assert_ne!(properties & MASK_CANPROP_DIR_TX, 0);
    }

    #[test]
    fn test_to_tlibcan_extended() {
        let msg = CanMessage::new_extended(0x12345678, &[5, 6, 7, 8]).unwrap();
        let libcan = to_tlibcan(&msg, 1).unwrap();

        // Copy fields to avoid unaligned references
        let idx_chn = libcan.FIdxChn;
        let identifier = libcan.FIdentifier;
        let dlc = libcan.FDLC;
        let properties = libcan.FProperties;

        assert_eq!(idx_chn, 1);
        assert_eq!(identifier, 0x12345678);
        assert_eq!(dlc, 4);
        assert_ne!(properties & MASK_CANPROP_EXTEND, 0);
    }

    #[test]
    fn test_from_tlibcan() {
        let libcan = TLIBCAN {
            FIdentifier: 0x456,
            FDLC: 3,
            FProperties: MASK_CANPROP_DIR_TX,
            FData: [0xAA, 0xBB, 0xCC, 0, 0, 0, 0, 0],
            ..Default::default()
        };

        let msg = from_tlibcan(&libcan);

        assert_eq!(msg.id(), CanId::Standard(0x456));
        assert_eq!(msg.data(), &[0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn test_dlc_to_len() {
        assert_eq!(dlc_to_len(0), 0);
        assert_eq!(dlc_to_len(8), 8);
        assert_eq!(dlc_to_len(9), 12);
        assert_eq!(dlc_to_len(10), 16);
        assert_eq!(dlc_to_len(15), 64);
    }

    #[test]
    fn test_len_to_dlc() {
        assert_eq!(len_to_dlc(0), 0);
        assert_eq!(len_to_dlc(8), 8);
        assert_eq!(len_to_dlc(12), 9);
        assert_eq!(len_to_dlc(16), 10);
        assert_eq!(len_to_dlc(64), 15);
    }

    #[test]
    fn test_rtr_flag() {
        // Create a remote frame using new_remote
        let msg = CanMessage::new_remote(CanId::Standard(0x100), 0).unwrap();

        let libcan = to_tlibcan(&msg, 0).unwrap();
        let properties = libcan.FProperties;
        assert_ne!(properties & MASK_CANPROP_REMOTE, 0);
    }
}
