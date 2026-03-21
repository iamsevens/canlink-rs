//! ISO-TP configuration types.

use super::frame::StMin;
use super::IsoTpError;
use std::time::Duration;

/// ISO-TP addressing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddressingMode {
    /// Normal addressing - CAN ID directly identifies the endpoint
    #[default]
    Normal,
    /// Extended addressing - first data byte is target address
    Extended {
        /// Target address byte
        target_address: u8,
    },
    /// Mixed addressing - 11-bit CAN ID + address extension byte
    Mixed {
        /// Address extension byte
        address_extension: u8,
    },
}

/// Frame size mode for ISO-TP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FrameSize {
    /// Auto-detect based on backend capabilities
    #[default]
    Auto,
    /// Force CAN 2.0 mode (8 bytes per frame)
    Classic8,
    /// Force CAN-FD mode (up to 64 bytes per frame)
    Fd64,
}

/// ISO-TP channel configuration.
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
    /// Flow Control block size (0 = no limit)
    pub block_size: u8,
    /// Flow Control `STmin`
    pub st_min: StMin,
    /// Receive timeout
    pub rx_timeout: Duration,
    /// Transmit timeout (waiting for FC)
    pub tx_timeout: Duration,
    /// Maximum FC(Wait) count before aborting
    pub max_wait_count: u8,
    /// Addressing mode
    pub addressing_mode: AddressingMode,
    /// Maximum buffer size for received data
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
            block_size: 0,
            st_min: StMin::Milliseconds(10),
            rx_timeout: Duration::from_millis(1000),
            tx_timeout: Duration::from_millis(1000),
            max_wait_count: 10,
            addressing_mode: AddressingMode::Normal,
            max_buffer_size: 4095,
            frame_size: FrameSize::Auto,
            padding_byte: 0xCC,
            padding_enabled: true,
        }
    }
}

impl IsoTpConfig {
    /// Create a configuration builder.
    #[must_use]
    pub fn builder() -> IsoTpConfigBuilder {
        IsoTpConfigBuilder::default()
    }

    /// Validate the configuration.
    ///
    /// # Errors
    ///
    /// Returns `IsoTpError::InvalidConfig` if any configuration value is invalid.
    pub fn validate(&self) -> Result<(), IsoTpError> {
        // Validate CAN IDs
        if !self.tx_extended && self.tx_id > 0x7FF {
            return Err(IsoTpError::InvalidConfig {
                reason: format!(
                    "TX ID 0x{:X} exceeds 11-bit limit for standard frame",
                    self.tx_id
                ),
            });
        }
        if self.tx_extended && self.tx_id > 0x1FFF_FFFF {
            return Err(IsoTpError::InvalidConfig {
                reason: format!(
                    "TX ID 0x{:X} exceeds 29-bit limit for extended frame",
                    self.tx_id
                ),
            });
        }
        if !self.rx_extended && self.rx_id > 0x7FF {
            return Err(IsoTpError::InvalidConfig {
                reason: format!(
                    "RX ID 0x{:X} exceeds 11-bit limit for standard frame",
                    self.rx_id
                ),
            });
        }
        if self.rx_extended && self.rx_id > 0x1FFF_FFFF {
            return Err(IsoTpError::InvalidConfig {
                reason: format!(
                    "RX ID 0x{:X} exceeds 29-bit limit for extended frame",
                    self.rx_id
                ),
            });
        }

        // Validate buffer size
        if self.max_buffer_size == 0 {
            return Err(IsoTpError::InvalidConfig {
                reason: "max_buffer_size cannot be 0".to_string(),
            });
        }
        if self.max_buffer_size > 4095 {
            return Err(IsoTpError::InvalidConfig {
                reason: format!(
                    "max_buffer_size {} exceeds ISO-TP limit of 4095",
                    self.max_buffer_size
                ),
            });
        }

        // Validate timeouts
        if self.rx_timeout.is_zero() {
            return Err(IsoTpError::InvalidConfig {
                reason: "rx_timeout cannot be zero".to_string(),
            });
        }
        if self.tx_timeout.is_zero() {
            return Err(IsoTpError::InvalidConfig {
                reason: "tx_timeout cannot be zero".to_string(),
            });
        }

        Ok(())
    }

    /// Get the maximum single frame data length based on frame size mode.
    #[must_use]
    pub fn max_sf_data_length(&self, is_fd: bool) -> usize {
        let base = match self.frame_size {
            FrameSize::Auto => {
                if is_fd {
                    62
                } else {
                    7
                }
            }
            FrameSize::Classic8 => 7,
            FrameSize::Fd64 => 62,
        };

        // Adjust for addressing mode overhead
        match self.addressing_mode {
            AddressingMode::Normal => base,
            AddressingMode::Extended { .. } | AddressingMode::Mixed { .. } => base - 1,
        }
    }

    /// Get the first frame data length based on frame size mode.
    #[must_use]
    pub fn ff_data_length(&self, is_fd: bool) -> usize {
        let base = match self.frame_size {
            FrameSize::Auto => {
                if is_fd {
                    62
                } else {
                    6
                }
            }
            FrameSize::Classic8 => 6,
            FrameSize::Fd64 => 62,
        };

        match self.addressing_mode {
            AddressingMode::Normal => base,
            AddressingMode::Extended { .. } | AddressingMode::Mixed { .. } => base - 1,
        }
    }

    /// Get the consecutive frame data length based on frame size mode.
    #[must_use]
    pub fn cf_data_length(&self, is_fd: bool) -> usize {
        let base = match self.frame_size {
            FrameSize::Auto => {
                if is_fd {
                    63
                } else {
                    7
                }
            }
            FrameSize::Classic8 => 7,
            FrameSize::Fd64 => 63,
        };

        match self.addressing_mode {
            AddressingMode::Normal => base,
            AddressingMode::Extended { .. } | AddressingMode::Mixed { .. } => base - 1,
        }
    }
}

/// Builder for ISO-TP configuration.
#[derive(Debug, Default)]
pub struct IsoTpConfigBuilder {
    config: IsoTpConfig,
}

impl IsoTpConfigBuilder {
    /// Set the transmit CAN ID.
    #[must_use]
    pub fn tx_id(mut self, id: u32) -> Self {
        self.config.tx_id = id;
        self
    }

    /// Set the receive CAN ID.
    #[must_use]
    pub fn rx_id(mut self, id: u32) -> Self {
        self.config.rx_id = id;
        self
    }

    /// Set whether to use extended IDs for both TX and RX.
    #[must_use]
    pub fn extended_ids(mut self, extended: bool) -> Self {
        self.config.tx_extended = extended;
        self.config.rx_extended = extended;
        self
    }

    /// Set the TX ID as extended.
    #[must_use]
    pub fn tx_extended(mut self, extended: bool) -> Self {
        self.config.tx_extended = extended;
        self
    }

    /// Set the RX ID as extended.
    #[must_use]
    pub fn rx_extended(mut self, extended: bool) -> Self {
        self.config.rx_extended = extended;
        self
    }

    /// Set the Flow Control block size.
    #[must_use]
    pub fn block_size(mut self, bs: u8) -> Self {
        self.config.block_size = bs;
        self
    }

    /// Set the Flow Control `STmin`.
    #[must_use]
    pub fn st_min(mut self, st_min: StMin) -> Self {
        self.config.st_min = st_min;
        self
    }

    /// Set both RX and TX timeout.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.rx_timeout = timeout;
        self.config.tx_timeout = timeout;
        self
    }

    /// Set the receive timeout.
    #[must_use]
    pub fn rx_timeout(mut self, timeout: Duration) -> Self {
        self.config.rx_timeout = timeout;
        self
    }

    /// Set the transmit timeout.
    #[must_use]
    pub fn tx_timeout(mut self, timeout: Duration) -> Self {
        self.config.tx_timeout = timeout;
        self
    }

    /// Set the maximum FC(Wait) count.
    #[must_use]
    pub fn max_wait_count(mut self, count: u8) -> Self {
        self.config.max_wait_count = count;
        self
    }

    /// Set the addressing mode.
    #[must_use]
    pub fn addressing_mode(mut self, mode: AddressingMode) -> Self {
        self.config.addressing_mode = mode;
        self
    }

    /// Set the maximum buffer size.
    #[must_use]
    pub fn max_buffer_size(mut self, size: usize) -> Self {
        self.config.max_buffer_size = size;
        self
    }

    /// Set the frame size mode.
    #[must_use]
    pub fn frame_size(mut self, size: FrameSize) -> Self {
        self.config.frame_size = size;
        self
    }

    /// Set the padding byte.
    #[must_use]
    pub fn padding_byte(mut self, byte: u8) -> Self {
        self.config.padding_byte = byte;
        self
    }

    /// Enable or disable padding.
    #[must_use]
    pub fn padding_enabled(mut self, enabled: bool) -> Self {
        self.config.padding_enabled = enabled;
        self
    }

    /// Build the configuration, validating it first.
    ///
    /// # Errors
    ///
    /// Returns `IsoTpError::InvalidConfig` if any configuration value is invalid.
    pub fn build(self) -> Result<IsoTpConfig, IsoTpError> {
        self.config.validate()?;
        Ok(self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = IsoTpConfig::default();
        assert_eq!(config.tx_id, 0);
        assert_eq!(config.rx_id, 0);
        assert!(!config.tx_extended);
        assert!(!config.rx_extended);
        assert_eq!(config.block_size, 0);
        assert_eq!(config.st_min, StMin::Milliseconds(10));
        assert_eq!(config.rx_timeout, Duration::from_millis(1000));
        assert_eq!(config.tx_timeout, Duration::from_millis(1000));
        assert_eq!(config.max_wait_count, 10);
        assert_eq!(config.addressing_mode, AddressingMode::Normal);
        assert_eq!(config.max_buffer_size, 4095);
        assert_eq!(config.frame_size, FrameSize::Auto);
        assert_eq!(config.padding_byte, 0xCC);
        assert!(config.padding_enabled);
    }

    #[test]
    fn test_builder() {
        let config = IsoTpConfig::builder()
            .tx_id(0x7E0)
            .rx_id(0x7E8)
            .block_size(8)
            .st_min(StMin::Milliseconds(20))
            .timeout(Duration::from_millis(500))
            .build()
            .unwrap();

        assert_eq!(config.tx_id, 0x7E0);
        assert_eq!(config.rx_id, 0x7E8);
        assert_eq!(config.block_size, 8);
        assert_eq!(config.st_min, StMin::Milliseconds(20));
        assert_eq!(config.rx_timeout, Duration::from_millis(500));
        assert_eq!(config.tx_timeout, Duration::from_millis(500));
    }

    #[test]
    fn test_builder_extended_ids() {
        let config = IsoTpConfig::builder()
            .tx_id(0x18DA_00F1)
            .rx_id(0x18DA_F100)
            .extended_ids(true)
            .build()
            .unwrap();

        assert!(config.tx_extended);
        assert!(config.rx_extended);
    }

    #[test]
    fn test_validation_standard_id_overflow() {
        let result = IsoTpConfig::builder()
            .tx_id(0x800) // > 0x7FF
            .rx_id(0x100)
            .build();

        assert!(matches!(result, Err(IsoTpError::InvalidConfig { .. })));
    }

    #[test]
    fn test_validation_extended_id_overflow() {
        let result = IsoTpConfig::builder()
            .tx_id(0x2000_0000) // > 0x1FFF_FFFF
            .rx_id(0x100)
            .extended_ids(true)
            .build();

        assert!(matches!(result, Err(IsoTpError::InvalidConfig { .. })));
    }

    #[test]
    fn test_validation_buffer_size_zero() {
        let result = IsoTpConfig::builder()
            .tx_id(0x7E0)
            .rx_id(0x7E8)
            .max_buffer_size(0)
            .build();

        assert!(matches!(result, Err(IsoTpError::InvalidConfig { .. })));
    }

    #[test]
    fn test_validation_buffer_size_too_large() {
        let result = IsoTpConfig::builder()
            .tx_id(0x7E0)
            .rx_id(0x7E8)
            .max_buffer_size(5000)
            .build();

        assert!(matches!(result, Err(IsoTpError::InvalidConfig { .. })));
    }

    #[test]
    fn test_validation_zero_timeout() {
        let result = IsoTpConfig::builder()
            .tx_id(0x7E0)
            .rx_id(0x7E8)
            .rx_timeout(Duration::ZERO)
            .build();

        assert!(matches!(result, Err(IsoTpError::InvalidConfig { .. })));
    }

    #[test]
    fn test_max_sf_data_length() {
        let config = IsoTpConfig::default();

        // CAN 2.0
        assert_eq!(config.max_sf_data_length(false), 7);
        // CAN-FD
        assert_eq!(config.max_sf_data_length(true), 62);

        // With extended addressing
        let config = IsoTpConfig::builder()
            .tx_id(0x7E0)
            .rx_id(0x7E8)
            .addressing_mode(AddressingMode::Extended {
                target_address: 0x01,
            })
            .build()
            .unwrap();

        assert_eq!(config.max_sf_data_length(false), 6);
        assert_eq!(config.max_sf_data_length(true), 61);
    }

    #[test]
    fn test_frame_size_forced() {
        let config = IsoTpConfig::builder()
            .tx_id(0x7E0)
            .rx_id(0x7E8)
            .frame_size(FrameSize::Classic8)
            .build()
            .unwrap();

        // Should always return CAN 2.0 sizes even if is_fd=true
        assert_eq!(config.max_sf_data_length(true), 7);
        assert_eq!(config.ff_data_length(true), 6);
        assert_eq!(config.cf_data_length(true), 7);
    }

    #[test]
    fn test_addressing_modes() {
        assert_eq!(AddressingMode::default(), AddressingMode::Normal);

        let extended = AddressingMode::Extended {
            target_address: 0x55,
        };
        if let AddressingMode::Extended { target_address } = extended {
            assert_eq!(target_address, 0x55);
        }

        let mixed = AddressingMode::Mixed {
            address_extension: 0xAA,
        };
        if let AddressingMode::Mixed { address_extension } = mixed {
            assert_eq!(address_extension, 0xAA);
        }
    }
}
