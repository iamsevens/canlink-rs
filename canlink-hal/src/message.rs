//! CAN message types and related structures.
//!
//! This module provides unified, hardware-independent representations of CAN messages,
//! identifiers, timestamps, and message flags.

use bitflags::bitflags;
use serde::{Deserialize, Serialize};

/// CAN identifier (standard or extended).
///
/// CAN supports two types of identifiers:
/// - Standard: 11-bit identifier (0x000-0x7FF)
/// - Extended: 29-bit identifier (0x00000000-0x1FFFFFFF)
///
/// # Examples
///
/// ```
/// use canlink_hal::CanId;
///
/// let std_id = CanId::Standard(0x123);
/// assert!(std_id.is_standard());
/// assert_eq!(std_id.raw(), 0x123);
///
/// let ext_id = CanId::Extended(0x12345678);
/// assert!(ext_id.is_extended());
/// assert_eq!(ext_id.raw(), 0x12345678);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CanId {
    /// Standard 11-bit ID (0x000-0x7FF)
    Standard(u16),

    /// Extended 29-bit ID (0x00000000-0x1FFFFFFF)
    Extended(u32),
}

impl CanId {
    /// Get the raw ID value as u32.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::CanId;
    ///
    /// assert_eq!(CanId::Standard(0x123).raw(), 0x123);
    /// assert_eq!(CanId::Extended(0x12345678).raw(), 0x12345678);
    /// ```
    #[must_use]
    pub fn raw(&self) -> u32 {
        match self {
            Self::Standard(id) => u32::from(*id),
            Self::Extended(id) => *id,
        }
    }

    /// Check if this is a standard frame ID.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::CanId;
    ///
    /// assert!(CanId::Standard(0x123).is_standard());
    /// assert!(!CanId::Extended(0x123).is_standard());
    /// ```
    #[must_use]
    pub fn is_standard(&self) -> bool {
        matches!(self, Self::Standard(_))
    }

    /// Check if this is an extended frame ID.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::CanId;
    ///
    /// assert!(!CanId::Standard(0x123).is_extended());
    /// assert!(CanId::Extended(0x123).is_extended());
    /// ```
    #[must_use]
    pub fn is_extended(&self) -> bool {
        matches!(self, Self::Extended(_))
    }
}

bitflags! {
    /// CAN message flags.
    ///
    /// These flags indicate various properties of a CAN message:
    /// - RTR: Remote Transmission Request
    /// - FD: CAN-FD format
    /// - BRS: Bit Rate Switch (CAN-FD only)
    /// - ESI: Error State Indicator (CAN-FD only)
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::MessageFlags;
    ///
    /// let flags = MessageFlags::FD | MessageFlags::BRS;
    /// assert!(flags.contains(MessageFlags::FD));
    /// assert!(flags.contains(MessageFlags::BRS));
    /// assert!(!flags.contains(MessageFlags::RTR));
    /// ```
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MessageFlags: u8 {
        /// Remote Transmission Request (RTR)
        const RTR = 0b0000_0001;

        /// CAN-FD format
        const FD = 0b0000_0010;

        /// Bit Rate Switch (BRS) - CAN-FD only
        const BRS = 0b0000_0100;

        /// Error State Indicator (ESI) - CAN-FD only
        const ESI = 0b0000_1000;
    }
}

impl Default for MessageFlags {
    fn default() -> Self {
        Self::empty()
    }
}

// Implement Serialize and Deserialize for MessageFlags
impl Serialize for MessageFlags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.bits().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MessageFlags {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bits = u8::deserialize(deserializer)?;
        Ok(MessageFlags::from_bits_truncate(bits))
    }
}

/// Microsecond-precision timestamp.
///
/// Represents a point in time with microsecond precision. The reference point
/// is hardware-dependent (typically system boot time or epoch).
///
/// # Examples
///
/// ```
/// use canlink_hal::Timestamp;
///
/// let ts = Timestamp::from_micros(1_000_000);
/// assert_eq!(ts.as_micros(), 1_000_000);
/// assert_eq!(ts.as_millis(), 1_000);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Timestamp {
    /// Microseconds since reference point
    micros: u64,
}

impl Timestamp {
    /// Create a timestamp from microseconds.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::Timestamp;
    ///
    /// let ts = Timestamp::from_micros(1_500_000);
    /// assert_eq!(ts.as_micros(), 1_500_000);
    /// ```
    #[must_use]
    pub const fn from_micros(micros: u64) -> Self {
        Self { micros }
    }

    /// Get the timestamp as microseconds.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::Timestamp;
    ///
    /// let ts = Timestamp::from_micros(1_500_000);
    /// assert_eq!(ts.as_micros(), 1_500_000);
    /// ```
    #[must_use]
    pub const fn as_micros(&self) -> u64 {
        self.micros
    }

    /// Get the timestamp as milliseconds.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::Timestamp;
    ///
    /// let ts = Timestamp::from_micros(1_500_000);
    /// assert_eq!(ts.as_millis(), 1_500);
    /// ```
    #[must_use]
    pub const fn as_millis(&self) -> u64 {
        self.micros / 1000
    }

    /// Get the timestamp as seconds.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::Timestamp;
    ///
    /// let ts = Timestamp::from_micros(2_500_000);
    /// assert_eq!(ts.as_secs(), 2);
    /// ```
    #[must_use]
    pub const fn as_secs(&self) -> u64 {
        self.micros / 1_000_000
    }
}

/// Unified CAN message type.
///
/// This structure represents a CAN message in a hardware-independent way.
/// It supports both CAN 2.0 and CAN-FD messages.
///
/// # Examples
///
/// ```
/// use canlink_hal::{CanMessage, CanId};
///
/// // Create a standard CAN 2.0 message
/// let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4]).unwrap();
/// assert_eq!(msg.id(), CanId::Standard(0x123));
/// assert_eq!(msg.data(), &[1, 2, 3, 4]);
///
/// // Create a CAN-FD message
/// let fd_msg = CanMessage::new_fd(CanId::Standard(0x456), &[1; 64]).unwrap();
/// assert_eq!(fd_msg.data().len(), 64);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanMessage {
    /// CAN identifier (standard or extended)
    id: CanId,

    /// Message data (up to 64 bytes for CAN-FD)
    data: Vec<u8>,

    /// Timestamp (microsecond precision)
    timestamp: Option<Timestamp>,

    /// Message flags
    flags: MessageFlags,
}

impl CanMessage {
    /// Create a standard CAN 2.0 data frame.
    ///
    /// # Arguments
    ///
    /// * `id` - Standard 11-bit CAN ID (0x000-0x7FF)
    /// * `data` - Data bytes (0-8 bytes)
    ///
    /// # Errors
    ///
    /// Returns `CanError::InvalidId` if the ID is out of range.
    /// Returns `CanError::InvalidDataLength` if data length exceeds 8 bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::CanMessage;
    ///
    /// let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4]).unwrap();
    /// assert_eq!(msg.data(), &[1, 2, 3, 4]);
    /// ```
    pub fn new_standard(id: u16, data: &[u8]) -> Result<Self, crate::error::CanError> {
        if id > 0x7FF {
            return Err(crate::error::CanError::InvalidId {
                value: u32::from(id),
                max: 0x7FF,
            });
        }
        if data.len() > 8 {
            return Err(crate::error::CanError::InvalidDataLength {
                expected: 8,
                actual: data.len(),
            });
        }
        Ok(Self {
            id: CanId::Standard(id),
            data: data.to_vec(),
            timestamp: None,
            flags: MessageFlags::default(),
        })
    }

    /// Create an extended CAN 2.0B data frame.
    ///
    /// # Arguments
    ///
    /// * `id` - Extended 29-bit CAN ID (0x00000000-0x1FFFFFFF)
    /// * `data` - Data bytes (0-8 bytes)
    ///
    /// # Errors
    ///
    /// Returns `CanError::InvalidId` if the ID is out of range.
    /// Returns `CanError::InvalidDataLength` if data length exceeds 8 bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::CanMessage;
    ///
    /// let msg = CanMessage::new_extended(0x12345678, &[1, 2, 3, 4]).unwrap();
    /// assert_eq!(msg.data(), &[1, 2, 3, 4]);
    /// ```
    pub fn new_extended(id: u32, data: &[u8]) -> Result<Self, crate::error::CanError> {
        if id > 0x1FFF_FFFF {
            return Err(crate::error::CanError::InvalidId {
                value: id,
                max: 0x1FFF_FFFF,
            });
        }
        if data.len() > 8 {
            return Err(crate::error::CanError::InvalidDataLength {
                expected: 8,
                actual: data.len(),
            });
        }
        Ok(Self {
            id: CanId::Extended(id),
            data: data.to_vec(),
            timestamp: None,
            flags: MessageFlags::default(),
        })
    }

    /// Create a CAN-FD data frame.
    ///
    /// # Arguments
    ///
    /// * `id` - CAN identifier (standard or extended)
    /// * `data` - Data bytes (0-64 bytes)
    ///
    /// # Errors
    ///
    /// Returns `CanError::InvalidDataLength` if data length exceeds 64 bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::{CanMessage, CanId};
    ///
    /// let msg = CanMessage::new_fd(CanId::Standard(0x123), &[1; 64]).unwrap();
    /// assert_eq!(msg.data().len(), 64);
    /// ```
    pub fn new_fd(id: CanId, data: &[u8]) -> Result<Self, crate::error::CanError> {
        if data.len() > 64 {
            return Err(crate::error::CanError::InvalidDataLength {
                expected: 64,
                actual: data.len(),
            });
        }
        Ok(Self {
            id,
            data: data.to_vec(),
            timestamp: None,
            flags: MessageFlags::FD | MessageFlags::BRS,
        })
    }

    /// Create a remote frame (RTR).
    ///
    /// # Arguments
    ///
    /// * `id` - CAN identifier (standard or extended)
    /// * `dlc` - Data Length Code (0-8)
    ///
    /// # Errors
    ///
    /// Returns `CanError::InvalidDataLength` if DLC exceeds 8.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::{CanMessage, CanId};
    ///
    /// let msg = CanMessage::new_remote(CanId::Standard(0x123), 4).unwrap();
    /// assert!(msg.is_remote());
    /// ```
    pub fn new_remote(id: CanId, dlc: u8) -> Result<Self, crate::error::CanError> {
        if dlc > 8 {
            return Err(crate::error::CanError::InvalidDataLength {
                expected: 8,
                actual: dlc as usize,
            });
        }
        Ok(Self {
            id,
            data: vec![],
            timestamp: None,
            flags: MessageFlags::RTR,
        })
    }

    /// Get the CAN identifier.
    #[must_use]
    pub const fn id(&self) -> CanId {
        self.id
    }

    /// Get the message data.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get the timestamp.
    #[must_use]
    pub const fn timestamp(&self) -> Option<Timestamp> {
        self.timestamp
    }

    /// Get the message flags.
    #[must_use]
    pub const fn flags(&self) -> MessageFlags {
        self.flags
    }

    /// Set the timestamp.
    pub fn set_timestamp(&mut self, timestamp: Timestamp) {
        self.timestamp = Some(timestamp);
    }

    /// Check if this is a remote frame.
    #[must_use]
    pub fn is_remote(&self) -> bool {
        self.flags.contains(MessageFlags::RTR)
    }

    /// Check if this is a CAN-FD frame.
    #[must_use]
    pub fn is_fd(&self) -> bool {
        self.flags.contains(MessageFlags::FD)
    }

    /// Check if this frame uses bit rate switching.
    #[must_use]
    pub fn is_brs(&self) -> bool {
        self.flags.contains(MessageFlags::BRS)
    }

    /// Check if this frame has error state indicator set.
    #[must_use]
    pub fn is_esi(&self) -> bool {
        self.flags.contains(MessageFlags::ESI)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_id_standard() {
        let id = CanId::Standard(0x123);
        assert!(id.is_standard());
        assert!(!id.is_extended());
        assert_eq!(id.raw(), 0x123);
    }

    #[test]
    fn test_can_id_extended() {
        let id = CanId::Extended(0x1234_5678);
        assert!(!id.is_standard());
        assert!(id.is_extended());
        assert_eq!(id.raw(), 0x1234_5678);
    }

    #[test]
    fn test_message_flags() {
        let flags = MessageFlags::FD | MessageFlags::BRS;
        assert!(flags.contains(MessageFlags::FD));
        assert!(flags.contains(MessageFlags::BRS));
        assert!(!flags.contains(MessageFlags::RTR));
    }

    #[test]
    fn test_timestamp() {
        let ts = Timestamp::from_micros(1_500_000);
        assert_eq!(ts.as_micros(), 1_500_000);
        assert_eq!(ts.as_millis(), 1_500);
        assert_eq!(ts.as_secs(), 1);
    }

    #[test]
    fn test_can_message_standard() {
        let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4]).unwrap();
        assert_eq!(msg.id(), CanId::Standard(0x123));
        assert_eq!(msg.data(), &[1, 2, 3, 4]);
        assert!(!msg.is_remote());
        assert!(!msg.is_fd());
    }

    #[test]
    fn test_can_message_extended() {
        let msg = CanMessage::new_extended(0x1234_5678, &[1, 2, 3, 4]).unwrap();
        assert_eq!(msg.id(), CanId::Extended(0x1234_5678));
        assert_eq!(msg.data(), &[1, 2, 3, 4]);
    }

    #[test]
    fn test_can_message_fd() {
        let msg = CanMessage::new_fd(CanId::Standard(0x123), &[1; 64]).unwrap();
        assert_eq!(msg.data().len(), 64);
        assert!(msg.is_fd());
        assert!(msg.is_brs());
    }

    #[test]
    fn test_can_message_remote() {
        let msg = CanMessage::new_remote(CanId::Standard(0x123), 4).unwrap();
        assert!(msg.is_remote());
        assert_eq!(msg.data().len(), 0);
    }

    #[test]
    fn test_invalid_standard_id() {
        let result = CanMessage::new_standard(0x800, &[1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_data_length() {
        let result = CanMessage::new_standard(0x123, &[1; 9]);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_fd_data_length() {
        let result = CanMessage::new_fd(CanId::Standard(0x123), &[1; 65]);
        assert!(result.is_err());
    }
}
