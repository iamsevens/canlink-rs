//! ISO-TP state machine types.

use std::time::Instant;

/// ISO-TP receive state.
#[derive(Debug, Default)]
pub enum RxState {
    /// Idle, waiting for SF or FF
    #[default]
    Idle,

    /// Receiving multi-frame message
    Receiving {
        /// Receive buffer
        buffer: Vec<u8>,
        /// Expected total length
        expected_length: usize,
        /// Next expected sequence number (0-15)
        next_sequence: u8,
        /// Frames received in current block
        block_count: u8,
        /// Reception start time
        start_time: Instant,
        /// Last frame receive time
        last_frame_time: Instant,
    },
}

impl RxState {
    /// Check if in idle state.
    #[must_use]
    pub fn is_idle(&self) -> bool {
        matches!(self, RxState::Idle)
    }

    /// Check if currently receiving.
    #[must_use]
    pub fn is_receiving(&self) -> bool {
        matches!(self, RxState::Receiving { .. })
    }

    /// Get the current buffer length if receiving.
    #[must_use]
    pub fn buffer_len(&self) -> Option<usize> {
        match self {
            RxState::Idle => None,
            RxState::Receiving { buffer, .. } => Some(buffer.len()),
        }
    }

    /// Get the expected total length if receiving.
    #[must_use]
    pub fn expected_length(&self) -> Option<usize> {
        match self {
            RxState::Idle => None,
            RxState::Receiving {
                expected_length, ..
            } => Some(*expected_length),
        }
    }

    /// Get the progress as a percentage (0-100) if receiving.
    #[must_use]
    pub fn progress_percent(&self) -> Option<u8> {
        match self {
            RxState::Idle => None,
            RxState::Receiving {
                buffer,
                expected_length,
                ..
            } => {
                if *expected_length == 0 {
                    Some(0)
                } else {
                    let progress = (buffer.len() * 100 / *expected_length).min(100);
                    Some(u8::try_from(progress).unwrap_or(100))
                }
            }
        }
    }
}

/// ISO-TP transmit state.
#[derive(Debug, Default)]
pub enum TxState {
    /// Idle
    #[default]
    Idle,

    /// Waiting for Flow Control after sending FF
    WaitingForFc {
        /// Data buffer to send
        buffer: Vec<u8>,
        /// Bytes already sent
        offset: usize,
        /// Next sequence number
        next_sequence: u8,
        /// Transmission start time
        start_time: Instant,
        /// Time when FC wait started
        fc_wait_start: Instant,
        /// Number of FC(Wait) responses received
        wait_count: u8,
    },

    /// Sending consecutive frames
    SendingCf {
        /// Data buffer to send
        buffer: Vec<u8>,
        /// Bytes already sent
        offset: usize,
        /// Next sequence number
        next_sequence: u8,
        /// Frames sent in current block
        block_count: u8,
        /// Block size limit (0 = no limit)
        block_size: u8,
        /// Minimum separation time
        st_min: std::time::Duration,
        /// Transmission start time
        start_time: Instant,
        /// Last frame send time
        last_frame_time: Instant,
    },
}

impl TxState {
    /// Check if in idle state.
    #[must_use]
    pub fn is_idle(&self) -> bool {
        matches!(self, TxState::Idle)
    }

    /// Check if waiting for flow control.
    #[must_use]
    pub fn is_waiting_for_fc(&self) -> bool {
        matches!(self, TxState::WaitingForFc { .. })
    }

    /// Check if sending consecutive frames.
    #[must_use]
    pub fn is_sending_cf(&self) -> bool {
        matches!(self, TxState::SendingCf { .. })
    }

    /// Check if currently sending (either waiting for FC or sending CF).
    #[must_use]
    pub fn is_sending(&self) -> bool {
        !self.is_idle()
    }

    /// Get the total buffer length if sending.
    #[must_use]
    pub fn buffer_len(&self) -> Option<usize> {
        match self {
            TxState::Idle => None,
            TxState::WaitingForFc { buffer, .. } | TxState::SendingCf { buffer, .. } => {
                Some(buffer.len())
            }
        }
    }

    /// Get the bytes sent so far if sending.
    #[must_use]
    pub fn bytes_sent(&self) -> Option<usize> {
        match self {
            TxState::Idle => None,
            TxState::WaitingForFc { offset, .. } | TxState::SendingCf { offset, .. } => {
                Some(*offset)
            }
        }
    }

    /// Get the progress as a percentage (0-100) if sending.
    #[must_use]
    pub fn progress_percent(&self) -> Option<u8> {
        match self {
            TxState::Idle => None,
            TxState::WaitingForFc { buffer, offset, .. }
            | TxState::SendingCf { buffer, offset, .. } => {
                if buffer.is_empty() {
                    Some(0)
                } else {
                    let progress = (*offset * 100 / buffer.len()).min(100);
                    Some(u8::try_from(progress).unwrap_or(100))
                }
            }
        }
    }
}

/// Combined ISO-TP channel state.
#[derive(Debug, Default)]
pub struct IsoTpState {
    /// Receive state
    pub rx: RxState,
    /// Transmit state
    pub tx: TxState,
}

impl IsoTpState {
    /// Create a new idle state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if both RX and TX are idle.
    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.rx.is_idle() && self.tx.is_idle()
    }

    /// Check if currently receiving.
    #[must_use]
    pub fn is_receiving(&self) -> bool {
        self.rx.is_receiving()
    }

    /// Check if currently sending.
    #[must_use]
    pub fn is_sending(&self) -> bool {
        self.tx.is_sending()
    }

    /// Reset both RX and TX to idle.
    pub fn reset(&mut self) {
        self.rx = RxState::Idle;
        self.tx = TxState::Idle;
    }

    /// Reset only RX to idle.
    pub fn reset_rx(&mut self) {
        self.rx = RxState::Idle;
    }

    /// Reset only TX to idle.
    pub fn reset_tx(&mut self) {
        self.tx = TxState::Idle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rx_state_idle() {
        let state = RxState::Idle;
        assert!(state.is_idle());
        assert!(!state.is_receiving());
        assert_eq!(state.buffer_len(), None);
        assert_eq!(state.expected_length(), None);
        assert_eq!(state.progress_percent(), None);
    }

    #[test]
    fn test_rx_state_receiving() {
        let now = Instant::now();
        let state = RxState::Receiving {
            buffer: vec![0x01, 0x02, 0x03, 0x04, 0x05],
            expected_length: 20,
            next_sequence: 2,
            block_count: 1,
            start_time: now,
            last_frame_time: now,
        };

        assert!(!state.is_idle());
        assert!(state.is_receiving());
        assert_eq!(state.buffer_len(), Some(5));
        assert_eq!(state.expected_length(), Some(20));
        assert_eq!(state.progress_percent(), Some(25)); // 5/20 = 25%
    }

    #[test]
    fn test_tx_state_idle() {
        let state = TxState::Idle;
        assert!(state.is_idle());
        assert!(!state.is_waiting_for_fc());
        assert!(!state.is_sending_cf());
        assert!(!state.is_sending());
        assert_eq!(state.buffer_len(), None);
        assert_eq!(state.bytes_sent(), None);
        assert_eq!(state.progress_percent(), None);
    }

    #[test]
    fn test_tx_state_waiting_for_fc() {
        let now = Instant::now();
        let state = TxState::WaitingForFc {
            buffer: vec![0; 100],
            offset: 6,
            next_sequence: 1,
            start_time: now,
            fc_wait_start: now,
            wait_count: 0,
        };

        assert!(!state.is_idle());
        assert!(state.is_waiting_for_fc());
        assert!(!state.is_sending_cf());
        assert!(state.is_sending());
        assert_eq!(state.buffer_len(), Some(100));
        assert_eq!(state.bytes_sent(), Some(6));
        assert_eq!(state.progress_percent(), Some(6)); // 6/100 = 6%
    }

    #[test]
    fn test_tx_state_sending_cf() {
        let now = Instant::now();
        let state = TxState::SendingCf {
            buffer: vec![0; 50],
            offset: 25,
            next_sequence: 4,
            block_count: 3,
            block_size: 8,
            st_min: std::time::Duration::from_millis(10),
            start_time: now,
            last_frame_time: now,
        };

        assert!(!state.is_idle());
        assert!(!state.is_waiting_for_fc());
        assert!(state.is_sending_cf());
        assert!(state.is_sending());
        assert_eq!(state.buffer_len(), Some(50));
        assert_eq!(state.bytes_sent(), Some(25));
        assert_eq!(state.progress_percent(), Some(50)); // 25/50 = 50%
    }

    #[test]
    fn test_isotp_state() {
        let mut state = IsoTpState::new();
        assert!(state.is_idle());
        assert!(!state.is_receiving());
        assert!(!state.is_sending());

        // Simulate receiving
        let now = Instant::now();
        state.rx = RxState::Receiving {
            buffer: vec![],
            expected_length: 100,
            next_sequence: 1,
            block_count: 0,
            start_time: now,
            last_frame_time: now,
        };

        assert!(!state.is_idle());
        assert!(state.is_receiving());
        assert!(!state.is_sending());

        // Reset RX only
        state.reset_rx();
        assert!(state.is_idle());

        // Simulate sending
        state.tx = TxState::WaitingForFc {
            buffer: vec![0; 50],
            offset: 6,
            next_sequence: 1,
            start_time: now,
            fc_wait_start: now,
            wait_count: 0,
        };

        assert!(!state.is_idle());
        assert!(!state.is_receiving());
        assert!(state.is_sending());

        // Full reset
        state.reset();
        assert!(state.is_idle());
    }

    #[test]
    fn test_progress_edge_cases() {
        let now = Instant::now();

        // Empty buffer
        let state = TxState::SendingCf {
            buffer: vec![],
            offset: 0,
            next_sequence: 0,
            block_count: 0,
            block_size: 0,
            st_min: std::time::Duration::ZERO,
            start_time: now,
            last_frame_time: now,
        };
        assert_eq!(state.progress_percent(), Some(0));

        // Zero expected length
        let state = RxState::Receiving {
            buffer: vec![],
            expected_length: 0,
            next_sequence: 0,
            block_count: 0,
            start_time: now,
            last_frame_time: now,
        };
        assert_eq!(state.progress_percent(), Some(0));

        // 100% complete
        let state = RxState::Receiving {
            buffer: vec![0; 100],
            expected_length: 100,
            next_sequence: 0,
            block_count: 0,
            start_time: now,
            last_frame_time: now,
        };
        assert_eq!(state.progress_percent(), Some(100));
    }
}
