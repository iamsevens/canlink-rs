//! ISO-TP (ISO 15765-2) API Contract
//!
//! This file defines the public API contract for ISO-TP transport protocol support.
//! Implementation must conform to these signatures and behaviors.
//!
//! # Feature: `isotp`
//!
//! Enable with:
//! ```toml
//! [dependencies]
//! canlink-hal = { version = "0.3", features = ["isotp"] }
//! ```

use crate::{CanBackendAsync, CanError, CanMessage};
use std::time::Duration;
use thiserror::Error;

// ============================================================================
// Constants
// ============================================================================

/// Maximum ISO-TP message size (ISO 15765-2 standard)
pub const MAX_MESSAGE_SIZE: usize = 4095;

/// Default receive timeout
pub const DEFAULT_RX_TIMEOUT: Duration = Duration::from_millis(1000);

/// Default transmit timeout (waiting for FC)
pub const DEFAULT_TX_TIMEOUT: Duration = Duration::from_millis(1000);

/// Default block size (0 = no limit)
pub const DEFAULT_BLOCK_SIZE: u8 = 0;

/// Default STmin (10ms)
pub const DEFAULT_ST_MIN_MS: u8 = 10;

/// CAN 2.0 frame data size
pub const CAN_CLASSIC_DATA_SIZE: usize = 8;

/// CAN-FD maximum frame data size
pub const CAN_FD_DATA_SIZE: usize = 64;

// ============================================================================
// IsoTpFrame
// ============================================================================

/// ISO-TP frame types.
///
/// # Frame Format (CAN 2.0)
///
/// ```text
/// Single Frame (SF):     [0x0N] [Data...]           N = length (1-7)
/// First Frame (FF):      [0x1L] [LL] [Data...]      Length = (L<<8)|LL
/// Consecutive Frame (CF):[0x2N] [Data...]           N = sequence (0-F)
/// Flow Control (FC):     [0x3S] [BS] [STmin] [pad]  S = status
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum IsoTpFrame {
    /// Single Frame - complete message in one frame (≤7 bytes for CAN 2.0)
    SingleFrame {
        /// Data length (1-7 for CAN 2.0, 1-62 for CAN-FD)
        data_length: u8,
        /// Message data
        data: Vec<u8>,
    },

    /// First Frame - first frame of a multi-frame message
    FirstFrame {
        /// Total message length (8-4095)
        total_length: u16,
        /// First chunk of data (6 bytes for CAN 2.0, 62 for CAN-FD)
        data: Vec<u8>,
    },

    /// Consecutive Frame - subsequent frames of a multi-frame message
    ConsecutiveFrame {
        /// Sequence number (0-15, wraps around)
        sequence_number: u8,
        /// Data chunk (7 bytes for CAN 2.0, 63 for CAN-FD)
        data: Vec<u8>,
    },

    /// Flow Control - receiver controls sender's transmission
    FlowControl {
        /// Flow status
        flow_status: FlowStatus,
        /// Block size (0 = send all remaining frames)
        block_size: u8,
        /// Minimum separation time
        st_min: StMin,
    },
}

impl IsoTpFrame {
    /// Decodes an ISO-TP frame from CAN message data.
    ///
    /// # Errors
    ///
    /// Returns `IsoTpError::InvalidFrame` if the data cannot be decoded.
    ///
    /// # Example
    ///
    /// ```rust
    /// use canlink_hal::isotp::IsoTpFrame;
    ///
    /// // Single Frame with 3 bytes of data
    /// let data = [0x03, 0x01, 0x02, 0x03, 0x00, 0x00, 0x00, 0x00];
    /// let frame = IsoTpFrame::decode(&data).unwrap();
    ///
    /// match frame {
    ///     IsoTpFrame::SingleFrame { data_length, data } => {
    ///         assert_eq!(data_length, 3);
    ///         assert_eq!(data, vec![0x01, 0x02, 0x03]);
    ///     }
    ///     _ => panic!("Expected SingleFrame"),
    /// }
    /// ```
    pub fn decode(data: &[u8]) -> Result<Self, IsoTpError>;

    /// Encodes the frame to CAN message data.
    ///
    /// # Example
    ///
    /// ```rust
    /// use canlink_hal::isotp::IsoTpFrame;
    ///
    /// let frame = IsoTpFrame::SingleFrame {
    ///     data_length: 3,
    ///     data: vec![0x01, 0x02, 0x03],
    /// };
    ///
    /// let encoded = frame.encode();
    /// assert_eq!(encoded[0], 0x03); // PCI: SF with length 3
    /// assert_eq!(&encoded[1..4], &[0x01, 0x02, 0x03]);
    /// ```
    pub fn encode(&self) -> Vec<u8>;

    /// Returns the PCI type nibble (0-3).
    pub fn pci_type(&self) -> u8;

    /// Returns true if this is a Single Frame.
    pub fn is_single_frame(&self) -> bool;

    /// Returns true if this is a First Frame.
    pub fn is_first_frame(&self) -> bool;

    /// Returns true if this is a Consecutive Frame.
    pub fn is_consecutive_frame(&self) -> bool;

    /// Returns true if this is a Flow Control frame.
    pub fn is_flow_control(&self) -> bool;
}

// ============================================================================
// FlowStatus
// ============================================================================

/// Flow Control status values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FlowStatus {
    /// Continue To Send - receiver ready for more frames
    ContinueToSend = 0x00,
    /// Wait - receiver temporarily not ready
    Wait = 0x01,
    /// Overflow - receiver buffer overflow, abort transfer
    Overflow = 0x02,
}

impl FlowStatus {
    /// Decodes from a byte value.
    ///
    /// # Errors
    ///
    /// Returns `IsoTpError::InvalidPci` if the value is not 0, 1, or 2.
    pub fn from_byte(byte: u8) -> Result<Self, IsoTpError>;

    /// Encodes to a byte value.
    pub fn to_byte(self) -> u8;
}

// ============================================================================
// StMin
// ============================================================================

/// Separation Time minimum encoding.
///
/// Per ISO 15765-2:
/// - 0x00-0x7F: 0-127 milliseconds
/// - 0xF1-0xF9: 100-900 microseconds
/// - Other values: reserved
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StMin {
    /// Milliseconds (0-127)
    Milliseconds(u8),
    /// Microseconds (100, 200, ..., 900)
    Microseconds(u16),
}

impl StMin {
    /// Decodes from a byte value.
    ///
    /// Reserved values are treated as 127ms (maximum).
    pub fn from_byte(byte: u8) -> Self;

    /// Encodes to a byte value.
    pub fn to_byte(self) -> u8;

    /// Converts to a Duration.
    pub fn to_duration(self) -> Duration;

    /// Creates from a Duration, choosing the closest valid encoding.
    pub fn from_duration(duration: Duration) -> Self;
}

impl Default for StMin {
    fn default() -> Self {
        StMin::Milliseconds(DEFAULT_ST_MIN_MS)
    }
}

// ============================================================================
// AddressingMode
// ============================================================================

/// ISO-TP addressing modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddressingMode {
    /// Normal addressing - CAN ID identifies the connection
    #[default]
    Normal,

    /// Extended addressing - first data byte is target address
    Extended {
        /// Target address byte
        target_address: u8,
    },

    /// Mixed addressing - 11-bit CAN ID with address extension
    Mixed {
        /// Address extension byte
        address_extension: u8,
    },
}

// ============================================================================
// FrameSize
// ============================================================================

/// Frame size mode for ISO-TP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FrameSize {
    /// Auto-detect based on backend CAN-FD capability
    #[default]
    Auto,

    /// Force CAN 2.0 classic mode (8 bytes per frame)
    Classic8,

    /// Force CAN-FD mode (up to 64 bytes per frame)
    Fd64,
}

// ============================================================================
// IsoTpConfig
// ============================================================================

/// Configuration for an ISO-TP channel.
///
/// # Example
///
/// ```rust
/// use canlink_hal::isotp::{IsoTpConfig, StMin, FrameSize};
/// use std::time::Duration;
///
/// let config = IsoTpConfig::builder()
///     .tx_id(0x7E0)
///     .rx_id(0x7E8)
///     .block_size(8)
///     .st_min(StMin::Milliseconds(5))
///     .timeout(Duration::from_millis(2000))
///     .frame_size(FrameSize::Auto)
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct IsoTpConfig {
    /// Transmit CAN ID
    pub tx_id: u32,
    /// Receive CAN ID
    pub rx_id: u32,
    /// Whether TX ID is extended (29-bit)
    pub tx_extended: bool,
    /// Whether RX ID is extended (29-bit)
    pub rx_extended: bool,
    /// Block size for Flow Control (0 = no limit)
    pub block_size: u8,
    /// STmin for Flow Control
    pub st_min: StMin,
    /// Receive timeout
    pub rx_timeout: Duration,
    /// Transmit timeout (waiting for FC)
    pub tx_timeout: Duration,
    /// Addressing mode
    pub addressing_mode: AddressingMode,
    /// Maximum buffer size
    pub max_buffer_size: usize,
    /// Frame size mode
    pub frame_size: FrameSize,
    /// Padding byte value
    pub padding_byte: u8,
    /// Whether to pad frames to full size
    pub padding_enabled: bool,
}

impl Default for IsoTpConfig {
    fn default() -> Self {
        Self {
            tx_id: 0,
            rx_id: 0,
            tx_extended: false,
            rx_extended: false,
            block_size: DEFAULT_BLOCK_SIZE,
            st_min: StMin::default(),
            rx_timeout: DEFAULT_RX_TIMEOUT,
            tx_timeout: DEFAULT_TX_TIMEOUT,
            addressing_mode: AddressingMode::Normal,
            max_buffer_size: MAX_MESSAGE_SIZE,
            frame_size: FrameSize::Auto,
            padding_byte: 0xCC,
            padding_enabled: true,
        }
    }
}

impl IsoTpConfig {
    /// Creates a configuration builder.
    pub fn builder() -> IsoTpConfigBuilder;

    /// Validates the configuration.
    ///
    /// # Errors
    ///
    /// Returns `IsoTpError::InvalidConfig` if:
    /// - tx_id or rx_id is invalid for the addressing mode
    /// - max_buffer_size > 4095
    pub fn validate(&self) -> Result<(), IsoTpError>;
}

/// Builder for IsoTpConfig.
#[derive(Debug, Default)]
pub struct IsoTpConfigBuilder {
    config: IsoTpConfig,
}

impl IsoTpConfigBuilder {
    /// Sets the transmit CAN ID.
    pub fn tx_id(self, id: u32) -> Self;

    /// Sets the receive CAN ID.
    pub fn rx_id(self, id: u32) -> Self;

    /// Sets whether to use extended (29-bit) IDs.
    pub fn extended_ids(self, extended: bool) -> Self;

    /// Sets the block size for Flow Control.
    pub fn block_size(self, bs: u8) -> Self;

    /// Sets the STmin for Flow Control.
    pub fn st_min(self, st_min: StMin) -> Self;

    /// Sets both RX and TX timeout.
    pub fn timeout(self, timeout: Duration) -> Self;

    /// Sets the RX timeout.
    pub fn rx_timeout(self, timeout: Duration) -> Self;

    /// Sets the TX timeout.
    pub fn tx_timeout(self, timeout: Duration) -> Self;

    /// Sets the addressing mode.
    pub fn addressing_mode(self, mode: AddressingMode) -> Self;

    /// Sets the maximum buffer size.
    pub fn max_buffer_size(self, size: usize) -> Self;

    /// Sets the frame size mode.
    pub fn frame_size(self, size: FrameSize) -> Self;

    /// Sets the padding byte.
    pub fn padding_byte(self, byte: u8) -> Self;

    /// Enables or disables padding.
    pub fn padding_enabled(self, enabled: bool) -> Self;

    /// Builds the configuration.
    ///
    /// # Errors
    ///
    /// Returns `IsoTpError::InvalidConfig` if validation fails.
    pub fn build(self) -> Result<IsoTpConfig, IsoTpError>;
}

// ============================================================================
// IsoTpChannel
// ============================================================================

/// An ISO-TP communication channel.
///
/// Handles automatic segmentation and reassembly of messages larger than
/// a single CAN frame, with Flow Control management.
///
/// # Example
///
/// ```rust,no_run
/// use canlink_hal::isotp::{IsoTpChannel, IsoTpConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let backend = /* create backend */;
///
///     let config = IsoTpConfig::builder()
///         .tx_id(0x7E0)
///         .rx_id(0x7E8)
///         .build()?;
///
///     let mut channel = IsoTpChannel::new(backend, config).await?;
///
///     // Send a large message (auto-segmented)
///     let data = vec![0x22, 0xF1, 0x90]; // UDS Read Data By Identifier
///     channel.send(&data).await?;
///
///     // Receive response (auto-reassembled)
///     let response = channel.receive().await?;
///     println!("Received {} bytes", response.len());
///
///     Ok(())
/// }
/// ```
pub struct IsoTpChannel<B: CanBackendAsync> {
    // Internal fields omitted from contract
}

impl<B: CanBackendAsync> IsoTpChannel<B> {
    /// Creates a new ISO-TP channel.
    ///
    /// # Arguments
    ///
    /// * `backend` - The CAN backend for sending/receiving frames
    /// * `config` - Channel configuration
    ///
    /// # Errors
    ///
    /// Returns `IsoTpError::InvalidConfig` if configuration is invalid.
    /// Returns `IsoTpError::BackendError` if backend capability query fails.
    pub async fn new(backend: B, config: IsoTpConfig) -> Result<Self, IsoTpError>;

    /// Sets a callback for transfer events.
    pub fn set_callback(&mut self, callback: Box<dyn IsoTpCallback>);

    /// Sends data using ISO-TP segmentation.
    ///
    /// For data ≤7 bytes (CAN 2.0) or ≤62 bytes (CAN-FD), sends as Single Frame.
    /// For larger data, sends as First Frame + Consecutive Frames with Flow Control.
    ///
    /// # Arguments
    ///
    /// * `data` - Data to send (1-4095 bytes)
    ///
    /// # Errors
    ///
    /// - `IsoTpError::EmptyData` if data is empty
    /// - `IsoTpError::DataTooLarge` if data exceeds 4095 bytes
    /// - `IsoTpError::FcTimeout` if Flow Control not received in time
    /// - `IsoTpError::RemoteOverflow` if receiver reports overflow
    /// - `IsoTpError::ChannelBusy` if a transfer is already in progress
    pub async fn send(&mut self, data: &[u8]) -> Result<(), IsoTpError>;

    /// Receives data using ISO-TP reassembly.
    ///
    /// Waits for a complete message (Single Frame or First Frame + all Consecutive Frames).
    /// Automatically sends Flow Control frames when receiving multi-frame messages.
    ///
    /// # Returns
    ///
    /// The complete reassembled message data.
    ///
    /// # Errors
    ///
    /// - `IsoTpError::RxTimeout` if message not received in time
    /// - `IsoTpError::SequenceMismatch` if CF sequence number is wrong
    /// - `IsoTpError::BufferOverflow` if message exceeds buffer size
    /// - `IsoTpError::ChannelBusy` if a transfer is already in progress
    pub async fn receive(&mut self) -> Result<Vec<u8>, IsoTpError>;

    /// Processes a received CAN message through the ISO-TP layer.
    ///
    /// Use this for manual message routing when not using automatic receive.
    ///
    /// # Returns
    ///
    /// `Some(data)` if a complete message was reassembled, `None` otherwise.
    pub async fn process_message(&mut self, message: &CanMessage) -> Result<Option<Vec<u8>>, IsoTpError>;

    /// Returns the current channel state.
    pub fn state(&self) -> &IsoTpState;

    /// Returns the channel configuration.
    pub fn config(&self) -> &IsoTpConfig;

    /// Aborts any in-progress transfer.
    pub fn abort(&mut self);

    /// Resets the channel to idle state.
    pub fn reset(&mut self);
}

// ============================================================================
// IsoTpState
// ============================================================================

/// ISO-TP channel state.
#[derive(Debug)]
pub struct IsoTpState {
    /// Receive state
    pub rx: RxState,
    /// Transmit state
    pub tx: TxState,
}

/// Receive state.
#[derive(Debug)]
pub enum RxState {
    /// Idle, waiting for SF or FF
    Idle,
    /// Receiving multi-frame message
    Receiving {
        /// Bytes received so far
        bytes_received: usize,
        /// Total expected bytes
        total_bytes: usize,
    },
}

/// Transmit state.
#[derive(Debug)]
pub enum TxState {
    /// Idle
    Idle,
    /// Waiting for Flow Control
    WaitingForFc,
    /// Sending Consecutive Frames
    SendingCf {
        /// Bytes sent so far
        bytes_sent: usize,
        /// Total bytes to send
        total_bytes: usize,
    },
}

impl IsoTpState {
    /// Returns true if the channel is idle (no transfer in progress).
    pub fn is_idle(&self) -> bool;

    /// Returns true if receiving a multi-frame message.
    pub fn is_receiving(&self) -> bool;

    /// Returns true if sending a multi-frame message.
    pub fn is_sending(&self) -> bool;
}

// ============================================================================
// IsoTpCallback
// ============================================================================

/// Callback trait for ISO-TP transfer events.
pub trait IsoTpCallback: Send + Sync {
    /// Called when a transfer starts.
    fn on_transfer_start(&self, direction: TransferDirection, total_length: usize);

    /// Called periodically during transfer.
    fn on_transfer_progress(&self, direction: TransferDirection, bytes_transferred: usize, total: usize);

    /// Called when a transfer completes successfully.
    fn on_transfer_complete(&self, direction: TransferDirection, data: &[u8]);

    /// Called when a transfer fails.
    fn on_transfer_error(&self, direction: TransferDirection, error: &IsoTpError);
}

/// Transfer direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDirection {
    /// Sending data
    Send,
    /// Receiving data
    Receive,
}

// ============================================================================
// IsoTpError
// ============================================================================

/// ISO-TP error types.
#[derive(Debug, Error)]
pub enum IsoTpError {
    /// Invalid frame format
    #[error("Invalid frame format: {reason}")]
    InvalidFrame { reason: String },

    /// Invalid PCI type
    #[error("Invalid PCI type: 0x{pci:02X}")]
    InvalidPci { pci: u8 },

    /// Sequence number mismatch
    #[error("Sequence number mismatch: expected {expected}, got {actual}")]
    SequenceMismatch { expected: u8, actual: u8 },

    /// Receive timeout
    #[error("Receive timeout after {timeout_ms}ms")]
    RxTimeout { timeout_ms: u64 },

    /// Flow Control timeout
    #[error("Timeout waiting for Flow Control after {timeout_ms}ms")]
    FcTimeout { timeout_ms: u64 },

    /// Buffer overflow
    #[error("Buffer overflow: received {received} bytes, max {max}")]
    BufferOverflow { received: usize, max: usize },

    /// Remote reported overflow
    #[error("Remote reported overflow")]
    RemoteOverflow,

    /// Data too large
    #[error("Data too large: {size} bytes, max {max}")]
    DataTooLarge { size: usize, max: usize },

    /// Empty data
    #[error("Data is empty")]
    EmptyData,

    /// Transfer aborted
    #[error("Transfer aborted")]
    Aborted,

    /// Invalid configuration
    #[error("Invalid configuration: {reason}")]
    InvalidConfig { reason: String },

    /// Backend error
    #[error("Backend error: {0}")]
    BackendError(#[from] CanError),

    /// Channel busy
    #[error("Channel busy: {state}")]
    ChannelBusy { state: String },

    /// Unexpected frame type
    #[error("Unexpected frame type: expected {expected}, got {actual}")]
    UnexpectedFrame { expected: String, actual: String },
}

// ============================================================================
// Tests (Contract Verification)
// ============================================================================

#[cfg(test)]
mod contract_tests {
    use super::*;

    /// FR-006: Frame encoding/decoding
    #[test]
    fn test_single_frame_codec() {
        let frame = IsoTpFrame::SingleFrame {
            data_length: 3,
            data: vec![0x01, 0x02, 0x03],
        };

        let encoded = frame.encode();
        assert_eq!(encoded[0] & 0xF0, 0x00); // SF PCI
        assert_eq!(encoded[0] & 0x0F, 3);    // Length

        let decoded = IsoTpFrame::decode(&encoded).unwrap();
        assert_eq!(frame, decoded);
    }

    #[test]
    fn test_first_frame_codec() {
        let frame = IsoTpFrame::FirstFrame {
            total_length: 100,
            data: vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06],
        };

        let encoded = frame.encode();
        assert_eq!(encoded[0] & 0xF0, 0x10); // FF PCI

        let decoded = IsoTpFrame::decode(&encoded).unwrap();
        assert_eq!(frame, decoded);
    }

    #[test]
    fn test_consecutive_frame_codec() {
        let frame = IsoTpFrame::ConsecutiveFrame {
            sequence_number: 5,
            data: vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07],
        };

        let encoded = frame.encode();
        assert_eq!(encoded[0] & 0xF0, 0x20); // CF PCI
        assert_eq!(encoded[0] & 0x0F, 5);    // Sequence

        let decoded = IsoTpFrame::decode(&encoded).unwrap();
        assert_eq!(frame, decoded);
    }

    #[test]
    fn test_flow_control_codec() {
        let frame = IsoTpFrame::FlowControl {
            flow_status: FlowStatus::ContinueToSend,
            block_size: 8,
            st_min: StMin::Milliseconds(10),
        };

        let encoded = frame.encode();
        assert_eq!(encoded[0] & 0xF0, 0x30); // FC PCI
        assert_eq!(encoded[1], 8);           // BS
        assert_eq!(encoded[2], 10);          // STmin

        let decoded = IsoTpFrame::decode(&encoded).unwrap();
        assert_eq!(frame, decoded);
    }

    /// FR-008: FC parameter encoding
    #[test]
    fn test_stmin_encoding() {
        // Milliseconds
        assert_eq!(StMin::Milliseconds(50).to_byte(), 50);
        assert_eq!(StMin::from_byte(50), StMin::Milliseconds(50));

        // Microseconds
        assert_eq!(StMin::Microseconds(500).to_byte(), 0xF5);
        assert_eq!(StMin::from_byte(0xF5), StMin::Microseconds(500));
    }

    /// FR-011: Timeout configuration
    #[test]
    fn test_config_defaults() {
        let config = IsoTpConfig::default();
        assert_eq!(config.rx_timeout, Duration::from_millis(1000));
        assert_eq!(config.tx_timeout, Duration::from_millis(1000));
        assert_eq!(config.block_size, 0);
        assert_eq!(config.max_buffer_size, 4095);
    }

    /// Edge case: Invalid frame
    #[test]
    fn test_invalid_frame() {
        // Empty data
        assert!(IsoTpFrame::decode(&[]).is_err());

        // Invalid PCI type (0x40+)
        assert!(IsoTpFrame::decode(&[0x40, 0x00, 0x00]).is_err());
    }

    /// Edge case: Data size limits
    #[test]
    fn test_data_size_limits() {
        let config = IsoTpConfig::default();
        assert!(config.validate().is_ok());

        let mut config = IsoTpConfig::default();
        config.max_buffer_size = 5000; // > 4095
        assert!(config.validate().is_err());
    }
}
