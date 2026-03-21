//! ISO-TP frame types and encoding/decoding.

use super::IsoTpError;

/// PCI type nibble values
const PCI_SINGLE_FRAME: u8 = 0x00;
const PCI_FIRST_FRAME: u8 = 0x10;
const PCI_CONSECUTIVE_FRAME: u8 = 0x20;
const PCI_FLOW_CONTROL: u8 = 0x30;

/// Flow Control status values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FlowStatus {
    /// Continue To Send - receiver is ready
    ContinueToSend = 0x00,
    /// Wait - receiver needs more time
    Wait = 0x01,
    /// Overflow - receiver buffer overflow, abort transfer
    Overflow = 0x02,
}

impl FlowStatus {
    /// Decode from byte.
    ///
    /// # Errors
    ///
    /// Returns `IsoTpError::InvalidFrame` when the flow status nibble is invalid.
    pub fn from_byte(byte: u8) -> Result<Self, IsoTpError> {
        match byte & 0x0F {
            0x00 => Ok(FlowStatus::ContinueToSend),
            0x01 => Ok(FlowStatus::Wait),
            0x02 => Ok(FlowStatus::Overflow),
            _ => Err(IsoTpError::InvalidFrame {
                reason: format!("invalid flow status: 0x{byte:02X}"),
            }),
        }
    }

    /// Encode to byte.
    #[must_use]
    pub fn to_byte(self) -> u8 {
        self as u8
    }
}

/// `STmin` (Separation Time minimum) encoding.
///
/// Specifies the minimum time between consecutive frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StMin {
    /// Milliseconds (0-127ms)
    Milliseconds(u8),
    /// Microseconds (100-900渭s, in steps of 100)
    Microseconds(u16),
}

impl StMin {
    /// Decode from byte.
    #[must_use]
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            0x00..=0x7F => StMin::Milliseconds(byte),
            0xF1..=0xF9 => StMin::Microseconds(u16::from(byte - 0xF0) * 100),
            // Reserved values treated as 127ms (max)
            _ => StMin::Milliseconds(127),
        }
    }

    /// Encode to byte.
    #[must_use]
    pub fn to_byte(self) -> u8 {
        match self {
            StMin::Milliseconds(ms) => {
                if ms > 127 {
                    127
                } else {
                    ms
                }
            }
            StMin::Microseconds(us) => {
                let hundreds = us / 100;
                if (1..=9).contains(&hundreds) {
                    let hundreds = u8::try_from(hundreds).unwrap_or(0);
                    0xF0 + hundreds
                } else {
                    // 0us => 0ms, >900us => round to 1ms
                    u8::from(hundreds != 0)
                }
            }
        }
    }

    /// Convert to Duration.
    #[must_use]
    pub fn to_duration(self) -> std::time::Duration {
        match self {
            StMin::Milliseconds(ms) => std::time::Duration::from_millis(u64::from(ms)),
            StMin::Microseconds(us) => std::time::Duration::from_micros(u64::from(us)),
        }
    }

    /// Create from Duration (chooses closest encoding).
    #[must_use]
    pub fn from_duration(duration: std::time::Duration) -> Self {
        let micros = duration.as_micros();
        if (100..=900).contains(&micros) {
            // Use microsecond encoding for 100-900渭s
            let rounded = u16::try_from((micros + 50) / 100 * 100).unwrap_or(u16::MAX);
            StMin::Microseconds(rounded.clamp(100, 900))
        } else {
            // Use millisecond encoding
            let millis = u8::try_from(duration.as_millis().min(127)).unwrap_or(127);
            StMin::Milliseconds(millis)
        }
    }
}

impl Default for StMin {
    fn default() -> Self {
        StMin::Milliseconds(10)
    }
}

/// ISO-TP frame types.
#[derive(Debug, Clone, PartialEq)]
pub enum IsoTpFrame {
    /// Single Frame - complete message in one frame
    SingleFrame {
        /// Data length (1-7 for CAN 2.0, 1-62 for CAN-FD)
        data_length: u8,
        /// Payload data
        data: Vec<u8>,
    },

    /// First Frame - start of multi-frame message
    FirstFrame {
        /// Total message length (8-4095)
        total_length: u16,
        /// First chunk of data (6 bytes for CAN 2.0, up to 62 for CAN-FD)
        data: Vec<u8>,
    },

    /// Consecutive Frame - continuation of multi-frame message
    ConsecutiveFrame {
        /// Sequence number (0-15, wraps around)
        sequence_number: u8,
        /// Data chunk (7 bytes for CAN 2.0, up to 63 for CAN-FD)
        data: Vec<u8>,
    },

    /// Flow Control - receiver feedback to sender
    FlowControl {
        /// Flow status
        flow_status: FlowStatus,
        /// Block size (0 = no limit)
        block_size: u8,
        /// Minimum separation time
        st_min: StMin,
    },
}

impl IsoTpFrame {
    /// Decode from CAN message data.
    ///
    /// # Errors
    ///
    /// Returns `IsoTpError::InvalidFrame` when data is malformed or incomplete.
    pub fn decode(data: &[u8]) -> Result<Self, IsoTpError> {
        if data.is_empty() {
            return Err(IsoTpError::InvalidFrame {
                reason: "empty data".to_string(),
            });
        }

        let pci = data[0] & 0xF0;

        match pci {
            PCI_SINGLE_FRAME => Self::decode_single_frame(data),
            PCI_FIRST_FRAME => Self::decode_first_frame(data),
            PCI_CONSECUTIVE_FRAME => Ok(Self::decode_consecutive_frame(data)),
            PCI_FLOW_CONTROL => Self::decode_flow_control(data),
            _ => Err(IsoTpError::InvalidPci { pci: data[0] }),
        }
    }

    fn decode_single_frame(data: &[u8]) -> Result<Self, IsoTpError> {
        let data_length = data[0] & 0x0F;

        if data_length == 0 {
            return Err(IsoTpError::InvalidFrame {
                reason: "SF data length is 0".to_string(),
            });
        }

        let payload_start = 1;
        let payload_end = payload_start + data_length as usize;

        if data.len() < payload_end {
            return Err(IsoTpError::InvalidFrame {
                reason: format!(
                    "SF too short: need {} bytes, got {}",
                    payload_end,
                    data.len()
                ),
            });
        }

        Ok(IsoTpFrame::SingleFrame {
            data_length,
            data: data[payload_start..payload_end].to_vec(),
        })
    }

    fn decode_first_frame(data: &[u8]) -> Result<Self, IsoTpError> {
        if data.len() < 2 {
            return Err(IsoTpError::InvalidFrame {
                reason: "FF too short".to_string(),
            });
        }

        let total_length = (u16::from(data[0] & 0x0F) << 8) | u16::from(data[1]);

        if total_length < 8 {
            return Err(IsoTpError::InvalidFrame {
                reason: format!("FF total length too small: {total_length}"),
            });
        }

        // First frame data starts at byte 2
        let payload = data[2..].to_vec();

        Ok(IsoTpFrame::FirstFrame {
            total_length,
            data: payload,
        })
    }

    fn decode_consecutive_frame(data: &[u8]) -> Self {
        let sequence_number = data[0] & 0x0F;

        // CF data starts at byte 1
        let payload = data[1..].to_vec();

        IsoTpFrame::ConsecutiveFrame {
            sequence_number,
            data: payload,
        }
    }

    fn decode_flow_control(data: &[u8]) -> Result<Self, IsoTpError> {
        if data.len() < 3 {
            return Err(IsoTpError::InvalidFrame {
                reason: format!("FC too short: need 3 bytes, got {}", data.len()),
            });
        }

        let flow_status = FlowStatus::from_byte(data[0])?;
        let block_size = data[1];
        let st_min = StMin::from_byte(data[2]);

        Ok(IsoTpFrame::FlowControl {
            flow_status,
            block_size,
            st_min,
        })
    }

    /// Encode to CAN message data.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        match self {
            IsoTpFrame::SingleFrame { data_length, data } => {
                let mut result = vec![PCI_SINGLE_FRAME | (*data_length & 0x0F)];
                result.extend_from_slice(data);
                result
            }

            IsoTpFrame::FirstFrame { total_length, data } => {
                let high = u8::try_from((*total_length >> 8) & 0x0F).unwrap_or(0);
                let low = u8::try_from(*total_length & 0xFF).unwrap_or(0);
                let mut result = vec![PCI_FIRST_FRAME | high, low];
                result.extend_from_slice(data);
                result
            }

            IsoTpFrame::ConsecutiveFrame {
                sequence_number,
                data,
            } => {
                let mut result = vec![PCI_CONSECUTIVE_FRAME | (*sequence_number & 0x0F)];
                result.extend_from_slice(data);
                result
            }

            IsoTpFrame::FlowControl {
                flow_status,
                block_size,
                st_min,
            } => {
                vec![
                    PCI_FLOW_CONTROL | flow_status.to_byte(),
                    *block_size,
                    st_min.to_byte(),
                ]
            }
        }
    }

    /// Get the PCI type nibble.
    #[must_use]
    pub fn pci_type(&self) -> u8 {
        match self {
            IsoTpFrame::SingleFrame { .. } => PCI_SINGLE_FRAME,
            IsoTpFrame::FirstFrame { .. } => PCI_FIRST_FRAME,
            IsoTpFrame::ConsecutiveFrame { .. } => PCI_CONSECUTIVE_FRAME,
            IsoTpFrame::FlowControl { .. } => PCI_FLOW_CONTROL,
        }
    }

    /// Check if this is a Single Frame.
    #[must_use]
    pub fn is_single_frame(&self) -> bool {
        matches!(self, IsoTpFrame::SingleFrame { .. })
    }

    /// Check if this is a First Frame.
    #[must_use]
    pub fn is_first_frame(&self) -> bool {
        matches!(self, IsoTpFrame::FirstFrame { .. })
    }

    /// Check if this is a Consecutive Frame.
    #[must_use]
    pub fn is_consecutive_frame(&self) -> bool {
        matches!(self, IsoTpFrame::ConsecutiveFrame { .. })
    }

    /// Check if this is a Flow Control frame.
    #[must_use]
    pub fn is_flow_control(&self) -> bool {
        matches!(self, IsoTpFrame::FlowControl { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flow_status_roundtrip() {
        assert_eq!(
            FlowStatus::from_byte(0x00).unwrap(),
            FlowStatus::ContinueToSend
        );
        assert_eq!(FlowStatus::from_byte(0x01).unwrap(), FlowStatus::Wait);
        assert_eq!(FlowStatus::from_byte(0x02).unwrap(), FlowStatus::Overflow);

        assert_eq!(FlowStatus::ContinueToSend.to_byte(), 0x00);
        assert_eq!(FlowStatus::Wait.to_byte(), 0x01);
        assert_eq!(FlowStatus::Overflow.to_byte(), 0x02);
    }

    #[test]
    fn test_flow_status_invalid() {
        assert!(FlowStatus::from_byte(0x03).is_err());
        assert!(FlowStatus::from_byte(0x0F).is_err());
    }

    #[test]
    fn test_stmin_milliseconds() {
        assert_eq!(StMin::from_byte(0x00), StMin::Milliseconds(0));
        assert_eq!(StMin::from_byte(0x7F), StMin::Milliseconds(127));
        assert_eq!(StMin::from_byte(0x50), StMin::Milliseconds(80));

        assert_eq!(StMin::Milliseconds(0).to_byte(), 0x00);
        assert_eq!(StMin::Milliseconds(127).to_byte(), 0x7F);
        assert_eq!(StMin::Milliseconds(200).to_byte(), 127); // Clamped
    }

    #[test]
    fn test_stmin_microseconds() {
        assert_eq!(StMin::from_byte(0xF1), StMin::Microseconds(100));
        assert_eq!(StMin::from_byte(0xF5), StMin::Microseconds(500));
        assert_eq!(StMin::from_byte(0xF9), StMin::Microseconds(900));

        assert_eq!(StMin::Microseconds(100).to_byte(), 0xF1);
        assert_eq!(StMin::Microseconds(500).to_byte(), 0xF5);
        assert_eq!(StMin::Microseconds(900).to_byte(), 0xF9);
    }

    #[test]
    fn test_stmin_reserved() {
        // Reserved values should be treated as 127ms
        assert_eq!(StMin::from_byte(0x80), StMin::Milliseconds(127));
        assert_eq!(StMin::from_byte(0xF0), StMin::Milliseconds(127));
        assert_eq!(StMin::from_byte(0xFF), StMin::Milliseconds(127));
    }

    #[test]
    fn test_stmin_duration() {
        use std::time::Duration;

        assert_eq!(
            StMin::Milliseconds(10).to_duration(),
            Duration::from_millis(10)
        );
        assert_eq!(
            StMin::Microseconds(500).to_duration(),
            Duration::from_micros(500)
        );

        assert_eq!(
            StMin::from_duration(Duration::from_millis(50)),
            StMin::Milliseconds(50)
        );
        assert_eq!(
            StMin::from_duration(Duration::from_micros(500)),
            StMin::Microseconds(500)
        );
    }

    #[test]
    fn test_single_frame_decode() {
        let data = [0x03, 0x01, 0x02, 0x03];
        let frame = IsoTpFrame::decode(&data).unwrap();

        match frame {
            IsoTpFrame::SingleFrame { data_length, data } => {
                assert_eq!(data_length, 3);
                assert_eq!(data, vec![0x01, 0x02, 0x03]);
            }
            _ => panic!("Expected SingleFrame"),
        }
    }

    #[test]
    fn test_single_frame_encode() {
        let frame = IsoTpFrame::SingleFrame {
            data_length: 3,
            data: vec![0x01, 0x02, 0x03],
        };
        let encoded = frame.encode();
        assert_eq!(encoded, vec![0x03, 0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_first_frame_decode() {
        let data = [0x10, 0x14, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        let frame = IsoTpFrame::decode(&data).unwrap();

        match frame {
            IsoTpFrame::FirstFrame { total_length, data } => {
                assert_eq!(total_length, 20);
                assert_eq!(data, vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
            }
            _ => panic!("Expected FirstFrame"),
        }
    }

    #[test]
    fn test_first_frame_encode() {
        let frame = IsoTpFrame::FirstFrame {
            total_length: 20,
            data: vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06],
        };
        let encoded = frame.encode();
        assert_eq!(
            encoded,
            vec![0x10, 0x14, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06]
        );
    }

    #[test]
    fn test_consecutive_frame_decode() {
        let data = [0x21, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D];
        let frame = IsoTpFrame::decode(&data).unwrap();

        match frame {
            IsoTpFrame::ConsecutiveFrame {
                sequence_number,
                data,
            } => {
                assert_eq!(sequence_number, 1);
                assert_eq!(data, vec![0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D]);
            }
            _ => panic!("Expected ConsecutiveFrame"),
        }
    }

    #[test]
    fn test_consecutive_frame_encode() {
        let frame = IsoTpFrame::ConsecutiveFrame {
            sequence_number: 5,
            data: vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07],
        };
        let encoded = frame.encode();
        assert_eq!(
            encoded,
            vec![0x25, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]
        );
    }

    #[test]
    fn test_flow_control_decode() {
        let data = [0x30, 0x08, 0x14]; // CTS, BS=8, STmin=20ms
        let frame = IsoTpFrame::decode(&data).unwrap();

        match frame {
            IsoTpFrame::FlowControl {
                flow_status,
                block_size,
                st_min,
            } => {
                assert_eq!(flow_status, FlowStatus::ContinueToSend);
                assert_eq!(block_size, 8);
                assert_eq!(st_min, StMin::Milliseconds(20));
            }
            _ => panic!("Expected FlowControl"),
        }
    }

    #[test]
    fn test_flow_control_encode() {
        let frame = IsoTpFrame::FlowControl {
            flow_status: FlowStatus::Wait,
            block_size: 0,
            st_min: StMin::Microseconds(500),
        };
        let encoded = frame.encode();
        assert_eq!(encoded, vec![0x31, 0x00, 0xF5]);
    }

    #[test]
    fn test_frame_type_checks() {
        let sf = IsoTpFrame::SingleFrame {
            data_length: 1,
            data: vec![0x00],
        };
        assert!(sf.is_single_frame());
        assert!(!sf.is_first_frame());
        assert!(!sf.is_consecutive_frame());
        assert!(!sf.is_flow_control());

        let ff = IsoTpFrame::FirstFrame {
            total_length: 20,
            data: vec![],
        };
        assert!(ff.is_first_frame());

        let cf = IsoTpFrame::ConsecutiveFrame {
            sequence_number: 0,
            data: vec![],
        };
        assert!(cf.is_consecutive_frame());

        let fc = IsoTpFrame::FlowControl {
            flow_status: FlowStatus::ContinueToSend,
            block_size: 0,
            st_min: StMin::default(),
        };
        assert!(fc.is_flow_control());
    }

    #[test]
    fn test_invalid_pci() {
        let data = [0x40, 0x00, 0x00]; // Invalid PCI type
        let result = IsoTpFrame::decode(&data);
        assert!(matches!(result, Err(IsoTpError::InvalidPci { pci: 0x40 })));
    }

    #[test]
    fn test_empty_data() {
        let result = IsoTpFrame::decode(&[]);
        assert!(matches!(result, Err(IsoTpError::InvalidFrame { .. })));
    }

    #[test]
    fn test_sf_zero_length() {
        let data = [0x00]; // SF with length 0
        let result = IsoTpFrame::decode(&data);
        assert!(matches!(result, Err(IsoTpError::InvalidFrame { .. })));
    }

    #[test]
    fn test_roundtrip() {
        let frames = vec![
            IsoTpFrame::SingleFrame {
                data_length: 5,
                data: vec![0x01, 0x02, 0x03, 0x04, 0x05],
            },
            IsoTpFrame::FirstFrame {
                total_length: 100,
                data: vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06],
            },
            IsoTpFrame::ConsecutiveFrame {
                sequence_number: 15,
                data: vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07],
            },
            IsoTpFrame::FlowControl {
                flow_status: FlowStatus::ContinueToSend,
                block_size: 16,
                st_min: StMin::Milliseconds(25),
            },
        ];

        for frame in frames {
            let encoded = frame.encode();
            let decoded = IsoTpFrame::decode(&encoded).unwrap();
            assert_eq!(frame, decoded);
        }
    }
}
