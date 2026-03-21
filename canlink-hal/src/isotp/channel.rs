//! ISO-TP channel implementation.

use super::config::{AddressingMode, IsoTpConfig};
use super::error::IsoTpError;
use super::frame::{FlowStatus, IsoTpFrame, StMin};
use super::state::{IsoTpState, RxState, TxState};
use crate::{CanBackendAsync, CanMessage};
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// Transfer direction for callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDirection {
    /// Receiving data
    Receive,
    /// Sending data
    Send,
}

/// ISO-TP transfer callback trait.
///
/// Implement this trait to receive notifications about transfer progress.
pub trait IsoTpCallback: Send {
    /// Called when a transfer starts.
    fn on_transfer_start(&mut self, direction: TransferDirection, total_length: usize) {
        let _ = (direction, total_length);
    }

    /// Called periodically during transfer with progress.
    fn on_transfer_progress(&mut self, direction: TransferDirection, bytes: usize, total: usize) {
        let _ = (direction, bytes, total);
    }

    /// Called when a transfer completes successfully.
    fn on_transfer_complete(&mut self, direction: TransferDirection, total_bytes: usize) {
        let _ = (direction, total_bytes);
    }

    /// Called when a transfer fails.
    fn on_transfer_error(&mut self, direction: TransferDirection, error: &IsoTpError) {
        let _ = (direction, error);
    }
}

/// No-op callback implementation.
#[derive(Debug, Default)]
pub struct NoOpCallback;

impl IsoTpCallback for NoOpCallback {}

/// ISO-TP channel for sending and receiving segmented messages.
///
/// This channel handles the ISO-TP protocol automatically:
/// - Single Frame (SF) for small messages
/// - First Frame (FF) + Consecutive Frames (CF) for large messages
/// - Automatic Flow Control (FC) handling
///
/// # Example
///
/// ```rust,ignore
/// use canlink_hal::isotp::{IsoTpChannel, IsoTpConfig};
///
/// let config = IsoTpConfig::builder()
///     .tx_id(0x7E0)
///     .rx_id(0x7E8)
///     .build()?;
///
/// let mut channel = IsoTpChannel::new(backend, config);
///
/// // Send data
/// channel.send(&[0x10, 0x01]).await?;
///
/// // Receive response
/// let response = channel.receive().await?;
/// ```
pub struct IsoTpChannel<B: CanBackendAsync> {
    /// CAN backend
    backend: B,
    /// Configuration
    config: IsoTpConfig,
    /// Channel state
    state: IsoTpState,
    /// Whether backend supports CAN-FD
    is_fd: bool,
    /// Optional callback
    callback: Option<Box<dyn IsoTpCallback>>,
}

impl<B: CanBackendAsync> IsoTpChannel<B> {
    fn duration_to_millis_u64(duration: Duration) -> u64 {
        u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
    }

    /// Create a new ISO-TP channel.
    ///
    /// # Arguments
    ///
    /// * `backend` - The CAN backend to use
    /// * `config` - Channel configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn new(backend: B, config: IsoTpConfig) -> Result<Self, IsoTpError> {
        config.validate()?;

        // Determine if we're using CAN-FD based on config
        // Auto defaults to CAN 2.0 (classic) - use Fd64 explicitly for CAN-FD
        let is_fd = matches!(config.frame_size, super::config::FrameSize::Fd64);

        Ok(Self {
            backend,
            config,
            state: IsoTpState::new(),
            is_fd,
            callback: None,
        })
    }

    /// Set a callback for transfer notifications.
    pub fn set_callback(&mut self, callback: impl IsoTpCallback + 'static) {
        self.callback = Some(Box::new(callback));
    }

    /// Get the current state.
    #[must_use]
    pub fn state(&self) -> &IsoTpState {
        &self.state
    }

    /// Check if the channel is idle.
    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.state.is_idle()
    }

    /// Abort any ongoing transfer.
    pub fn abort(&mut self) {
        if let Some(ref mut cb) = self.callback {
            if self.state.is_receiving() {
                cb.on_transfer_error(TransferDirection::Receive, &IsoTpError::Aborted);
            }
            if self.state.is_sending() {
                cb.on_transfer_error(TransferDirection::Send, &IsoTpError::Aborted);
            }
        }
        self.state.reset();
    }

    /// Get the maximum single frame data length.
    fn max_sf_len(&self) -> usize {
        self.config.max_sf_data_length(self.is_fd)
    }

    /// Get the first frame data length.
    fn ff_data_len(&self) -> usize {
        self.config.ff_data_length(self.is_fd)
    }

    /// Get the consecutive frame data length.
    fn cf_data_len(&self) -> usize {
        self.config.cf_data_length(self.is_fd)
    }

    /// Get the address byte for Extended/Mixed addressing modes.
    /// Returns None for Normal addressing.
    fn get_address_byte(&self) -> Option<u8> {
        match self.config.addressing_mode {
            AddressingMode::Normal => None,
            AddressingMode::Extended { target_address } => Some(target_address),
            AddressingMode::Mixed { address_extension } => Some(address_extension),
        }
    }

    /// Prepend address byte to frame data for Extended/Mixed addressing.
    fn prepend_address_byte(&self, mut data: Vec<u8>) -> Vec<u8> {
        if let Some(addr) = self.get_address_byte() {
            data.insert(0, addr);
        }
        data
    }

    /// Strip and validate address byte from received frame data.
    /// Returns the remaining data after the address byte, or error if address doesn't match.
    fn strip_address_byte<'a>(&self, data: &'a [u8]) -> Result<&'a [u8], IsoTpError> {
        match self.config.addressing_mode {
            AddressingMode::Normal => Ok(data),
            AddressingMode::Extended { target_address } => {
                if data.is_empty() {
                    return Err(IsoTpError::InvalidFrame {
                        reason: "frame too short for extended addressing".to_string(),
                    });
                }
                // In extended addressing, we receive frames with our address
                // The first byte should match our expected source address
                // For simplicity, we accept any address byte and just strip it
                // (In a full implementation, you might want to validate against expected source)
                let _ = target_address; // We use target_address for TX, not RX validation
                Ok(&data[1..])
            }
            AddressingMode::Mixed { address_extension } => {
                if data.is_empty() {
                    return Err(IsoTpError::InvalidFrame {
                        reason: "frame too short for mixed addressing".to_string(),
                    });
                }
                // In mixed addressing, validate the address extension byte
                if data[0] != address_extension {
                    return Err(IsoTpError::InvalidFrame {
                        reason: format!(
                            "address extension mismatch: expected 0x{:02X}, got 0x{:02X}",
                            address_extension, data[0]
                        ),
                    });
                }
                Ok(&data[1..])
            }
        }
    }

    /// Create a CAN message for transmission.
    fn create_tx_message(&self, data: Vec<u8>) -> Result<CanMessage, IsoTpError> {
        // Prepend address byte for Extended/Mixed addressing
        let mut frame_data = self.prepend_address_byte(data);

        // Apply padding if enabled
        if self.config.padding_enabled {
            let target_len = if self.is_fd { 64 } else { 8 };
            while frame_data.len() < target_len {
                frame_data.push(self.config.padding_byte);
            }
        }

        let msg = if self.config.tx_extended {
            CanMessage::new_extended(self.config.tx_id, &frame_data)
        } else {
            // Standard ID is 11-bit, config validation ensures tx_id <= 0x7FF
            let tx_id =
                u16::try_from(self.config.tx_id).map_err(|_| IsoTpError::InvalidConfig {
                    reason: format!("tx_id out of range: 0x{:X}", self.config.tx_id),
                })?;
            CanMessage::new_standard(tx_id, &frame_data)
        }
        .map_err(IsoTpError::BackendError)?;

        Ok(msg)
    }

    /// Send data using ISO-TP protocol.
    ///
    /// For data ≤ 7 bytes (CAN 2.0) or ≤ 62 bytes (CAN-FD), sends as Single Frame.
    /// For larger data, sends as First Frame + Consecutive Frames with Flow Control.
    ///
    /// # Arguments
    ///
    /// * `data` - Data to send (1-4095 bytes)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Data is empty or too large
    /// - Channel is busy
    /// - Backend error occurs
    /// - Flow Control timeout
    /// - Remote reports overflow
    pub async fn send(&mut self, data: &[u8]) -> Result<(), IsoTpError> {
        // Validate data
        if data.is_empty() {
            return Err(IsoTpError::EmptyData);
        }
        if data.len() > self.config.max_buffer_size {
            return Err(IsoTpError::DataTooLarge {
                size: data.len(),
                max: self.config.max_buffer_size,
            });
        }

        // Check if channel is busy
        if !self.state.tx.is_idle() {
            return Err(IsoTpError::ChannelBusy {
                state: "sending".to_string(),
            });
        }

        // Notify callback
        if let Some(ref mut cb) = self.callback {
            cb.on_transfer_start(TransferDirection::Send, data.len());
        }

        // Single Frame for small data
        if data.len() <= self.max_sf_len() {
            return self.send_single_frame(data).await;
        }

        // Multi-frame transfer
        self.send_multi_frame(data).await
    }

    /// Send a single frame.
    async fn send_single_frame(&mut self, data: &[u8]) -> Result<(), IsoTpError> {
        let data_length = u8::try_from(data.len()).map_err(|_| IsoTpError::DataTooLarge {
            size: data.len(),
            max: self.max_sf_len(),
        })?;
        let frame = IsoTpFrame::SingleFrame {
            data_length,
            data: data.to_vec(),
        };

        let msg = self.create_tx_message(frame.encode())?;
        self.backend
            .send_message_async(&msg)
            .await
            .map_err(IsoTpError::BackendError)?;

        if let Some(ref mut cb) = self.callback {
            cb.on_transfer_complete(TransferDirection::Send, data.len());
        }

        Ok(())
    }

    /// Send a multi-frame message.
    async fn send_multi_frame(&mut self, data: &[u8]) -> Result<(), IsoTpError> {
        let now = Instant::now();

        // Send First Frame
        let ff_data_len = self.ff_data_len().min(data.len());
        let total_length = u16::try_from(data.len()).map_err(|_| IsoTpError::DataTooLarge {
            size: data.len(),
            max: self.config.max_buffer_size,
        })?;
        let frame = IsoTpFrame::FirstFrame {
            total_length,
            data: data[..ff_data_len].to_vec(),
        };

        let msg = self.create_tx_message(frame.encode())?;
        self.backend
            .send_message_async(&msg)
            .await
            .map_err(IsoTpError::BackendError)?;

        // Set state to waiting for FC
        self.state.tx = TxState::WaitingForFc {
            buffer: data.to_vec(),
            offset: ff_data_len,
            next_sequence: 1,
            start_time: now,
            fc_wait_start: now,
            wait_count: 0,
        };

        // Wait for FC and send CFs
        self.continue_send().await
    }

    /// Continue sending after receiving FC.
    async fn continue_send(&mut self) -> Result<(), IsoTpError> {
        loop {
            match &self.state.tx {
                TxState::Idle => {
                    return Ok(());
                }
                TxState::WaitingForFc { .. } => {
                    self.wait_for_flow_control().await?;
                }
                TxState::SendingCf { .. } => {
                    if self.send_consecutive_frames().await? {
                        // Transfer complete
                        return Ok(());
                    }
                }
            }
        }
    }

    /// Wait for Flow Control frame.
    async fn wait_for_flow_control(&mut self) -> Result<(), IsoTpError> {
        let fc_timeout = self.config.tx_timeout;

        loop {
            // Receive with timeout
            let result = timeout(fc_timeout, self.receive_frame()).await;

            match result {
                Ok(Ok(frame)) => {
                    if let IsoTpFrame::FlowControl {
                        flow_status,
                        block_size,
                        st_min,
                    } = frame
                    {
                        match flow_status {
                            FlowStatus::ContinueToSend => {
                                // Transition to SendingCf
                                if let TxState::WaitingForFc {
                                    buffer,
                                    offset,
                                    next_sequence,
                                    start_time,
                                    ..
                                } = std::mem::take(&mut self.state.tx)
                                {
                                    self.state.tx = TxState::SendingCf {
                                        buffer,
                                        offset,
                                        next_sequence,
                                        block_count: 0,
                                        block_size,
                                        st_min: st_min.to_duration(),
                                        start_time,
                                        last_frame_time: Instant::now(),
                                    };
                                }
                                return Ok(());
                            }
                            FlowStatus::Wait => {
                                // Increment wait count and check limit
                                let exceeded = if let TxState::WaitingForFc { wait_count, .. } =
                                    &mut self.state.tx
                                {
                                    *wait_count += 1;
                                    *wait_count > self.config.max_wait_count
                                } else {
                                    false
                                };

                                if exceeded {
                                    let count = self.config.max_wait_count + 1;
                                    self.state.reset_tx();
                                    let err = IsoTpError::TooManyWaits {
                                        count,
                                        max: self.config.max_wait_count,
                                    };
                                    if let Some(ref mut cb) = self.callback {
                                        cb.on_transfer_error(TransferDirection::Send, &err);
                                    }
                                    return Err(err);
                                }
                                // Continue waiting
                            }
                            FlowStatus::Overflow => {
                                self.state.reset_tx();
                                if let Some(ref mut cb) = self.callback {
                                    cb.on_transfer_error(
                                        TransferDirection::Send,
                                        &IsoTpError::RemoteOverflow,
                                    );
                                }
                                return Err(IsoTpError::RemoteOverflow);
                            }
                        }
                    }
                    // Ignore non-FC frames (per spec, scenario 3.5)
                    #[cfg(feature = "tracing")]
                    tracing::debug!("Ignoring non-FC frame while waiting for FC");
                }
                Ok(Err(e)) => {
                    self.state.reset_tx();
                    if let Some(ref mut cb) = self.callback {
                        cb.on_transfer_error(TransferDirection::Send, &e);
                    }
                    return Err(e);
                }
                Err(_) => {
                    // Timeout
                    self.state.reset_tx();
                    let err = IsoTpError::FcTimeout {
                        timeout_ms: Self::duration_to_millis_u64(fc_timeout),
                    };
                    if let Some(ref mut cb) = self.callback {
                        cb.on_transfer_error(TransferDirection::Send, &err);
                    }
                    return Err(err);
                }
            }
        }
    }

    /// Send consecutive frames. Returns true when transfer is complete.
    async fn send_consecutive_frames(&mut self) -> Result<bool, IsoTpError> {
        // Extract state
        let (
            buffer,
            mut offset,
            mut next_sequence,
            mut block_count,
            block_size,
            st_min,
            start_time,
        ) = if let TxState::SendingCf {
            buffer,
            offset,
            next_sequence,
            block_count,
            block_size,
            st_min,
            start_time,
            ..
        } = &self.state.tx
        {
            (
                buffer.clone(),
                *offset,
                *next_sequence,
                *block_count,
                *block_size,
                *st_min,
                *start_time,
            )
        } else {
            return Ok(true);
        };

        let cf_data_len = self.cf_data_len();

        // Send CFs until block complete or data exhausted
        while offset < buffer.len() {
            // Check block size limit
            if block_size > 0 && block_count >= block_size {
                // Need to wait for next FC
                self.state.tx = TxState::WaitingForFc {
                    buffer,
                    offset,
                    next_sequence,
                    start_time,
                    fc_wait_start: Instant::now(),
                    wait_count: 0,
                };
                return Ok(false);
            }

            // Wait for STmin
            if !st_min.is_zero() {
                tokio::time::sleep(st_min).await;
            }

            // Build CF
            let end = (offset + cf_data_len).min(buffer.len());
            let frame = IsoTpFrame::ConsecutiveFrame {
                sequence_number: next_sequence,
                data: buffer[offset..end].to_vec(),
            };

            let msg = self.create_tx_message(frame.encode())?;
            self.backend
                .send_message_async(&msg)
                .await
                .map_err(IsoTpError::BackendError)?;

            offset = end;
            next_sequence = (next_sequence + 1) & 0x0F;
            block_count += 1;

            // Progress callback
            if let Some(ref mut cb) = self.callback {
                cb.on_transfer_progress(TransferDirection::Send, offset, buffer.len());
            }
        }

        // Transfer complete
        self.state.reset_tx();
        if let Some(ref mut cb) = self.callback {
            cb.on_transfer_complete(TransferDirection::Send, buffer.len());
        }

        Ok(true)
    }

    /// Receive data using ISO-TP protocol.
    ///
    /// Waits for incoming ISO-TP message and returns the complete data.
    /// Automatically handles Flow Control for multi-frame messages.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Channel is busy
    /// - Receive timeout
    /// - Sequence number mismatch
    /// - Buffer overflow
    pub async fn receive(&mut self) -> Result<Vec<u8>, IsoTpError> {
        // Check if channel is busy
        if !self.state.rx.is_idle() {
            return Err(IsoTpError::ChannelBusy {
                state: "receiving".to_string(),
            });
        }

        self.receive_message().await
    }

    /// Internal receive implementation.
    async fn receive_message(&mut self) -> Result<Vec<u8>, IsoTpError> {
        let rx_timeout = self.config.rx_timeout;

        loop {
            // Receive with timeout
            let result = timeout(rx_timeout, self.receive_frame()).await;

            match result {
                Ok(Ok(frame)) => {
                    if let Some(data) = self.process_rx_frame(frame).await? {
                        return Ok(data);
                    }
                }
                Ok(Err(e)) => {
                    self.state.reset_rx();
                    if let Some(ref mut cb) = self.callback {
                        cb.on_transfer_error(TransferDirection::Receive, &e);
                    }
                    return Err(e);
                }
                Err(_) => {
                    // Timeout
                    let was_receiving = self.state.rx.is_receiving();
                    self.state.reset_rx();
                    let err = IsoTpError::RxTimeout {
                        timeout_ms: Self::duration_to_millis_u64(rx_timeout),
                    };
                    if was_receiving {
                        if let Some(ref mut cb) = self.callback {
                            cb.on_transfer_error(TransferDirection::Receive, &err);
                        }
                    }
                    return Err(err);
                }
            }
        }
    }

    /// Process a received frame. Returns Some(data) when message is complete.
    #[allow(clippy::too_many_lines)]
    async fn process_rx_frame(&mut self, frame: IsoTpFrame) -> Result<Option<Vec<u8>>, IsoTpError> {
        match frame {
            IsoTpFrame::SingleFrame { data, .. } => {
                if self.state.rx.is_receiving() {
                    // Unexpected SF while receiving - abort current and return error
                    self.state.reset_rx();
                    return Err(IsoTpError::UnexpectedFrame {
                        expected: "CF".to_string(),
                        actual: "SF".to_string(),
                    });
                }

                if let Some(ref mut cb) = self.callback {
                    cb.on_transfer_start(TransferDirection::Receive, data.len());
                    cb.on_transfer_complete(TransferDirection::Receive, data.len());
                }

                Ok(Some(data))
            }

            IsoTpFrame::FirstFrame { total_length, data } => {
                if self.state.rx.is_receiving() {
                    // New FF while receiving - send FC(Overflow) and abort
                    self.send_flow_control(FlowStatus::Overflow, 0, StMin::default())
                        .await?;
                    self.state.reset_rx();
                    return Err(IsoTpError::UnexpectedFrame {
                        expected: "CF".to_string(),
                        actual: "FF".to_string(),
                    });
                }

                let total_length = total_length as usize;

                // Check buffer size
                if total_length > self.config.max_buffer_size {
                    self.send_flow_control(FlowStatus::Overflow, 0, StMin::default())
                        .await?;
                    return Err(IsoTpError::BufferOverflow {
                        received: total_length,
                        max: self.config.max_buffer_size,
                    });
                }

                // Start receiving
                let now = Instant::now();
                let mut buffer = Vec::with_capacity(total_length);
                buffer.extend_from_slice(&data);

                self.state.rx = RxState::Receiving {
                    buffer,
                    expected_length: total_length,
                    next_sequence: 1,
                    block_count: 0,
                    start_time: now,
                    last_frame_time: now,
                };

                if let Some(ref mut cb) = self.callback {
                    cb.on_transfer_start(TransferDirection::Receive, total_length);
                    cb.on_transfer_progress(TransferDirection::Receive, data.len(), total_length);
                }

                // Send Flow Control
                self.send_flow_control(
                    FlowStatus::ContinueToSend,
                    self.config.block_size,
                    self.config.st_min,
                )
                .await?;

                Ok(None)
            }

            IsoTpFrame::ConsecutiveFrame {
                sequence_number,
                data,
            } => {
                if let RxState::Receiving {
                    buffer,
                    expected_length,
                    next_sequence,
                    block_count,
                    ..
                } = &mut self.state.rx
                {
                    // Check sequence number
                    if sequence_number != *next_sequence {
                        let expected = *next_sequence;
                        self.state.reset_rx();
                        return Err(IsoTpError::SequenceMismatch {
                            expected,
                            actual: sequence_number,
                        });
                    }

                    // Append data
                    let remaining = *expected_length - buffer.len();
                    let to_copy = data.len().min(remaining);
                    buffer.extend_from_slice(&data[..to_copy]);

                    *next_sequence = (*next_sequence + 1) & 0x0F;
                    *block_count += 1;

                    // Progress callback
                    if let Some(ref mut cb) = self.callback {
                        cb.on_transfer_progress(
                            TransferDirection::Receive,
                            buffer.len(),
                            *expected_length,
                        );
                    }

                    // Check if complete
                    if buffer.len() >= *expected_length {
                        let result = buffer.clone();
                        self.state.reset_rx();

                        if let Some(ref mut cb) = self.callback {
                            cb.on_transfer_complete(TransferDirection::Receive, result.len());
                        }

                        return Ok(Some(result));
                    }

                    // Check if need to send FC for next block
                    if self.config.block_size > 0 && *block_count >= self.config.block_size {
                        *block_count = 0;
                        self.send_flow_control(
                            FlowStatus::ContinueToSend,
                            self.config.block_size,
                            self.config.st_min,
                        )
                        .await?;
                    }

                    Ok(None)
                } else {
                    // CF without FF - ignore
                    #[cfg(feature = "tracing")]
                    tracing::warn!("Received CF without active reception");
                    Ok(None)
                }
            }

            IsoTpFrame::FlowControl { .. } => {
                // FC is handled in send path, ignore here
                Ok(None)
            }
        }
    }

    /// Send a Flow Control frame.
    async fn send_flow_control(
        &mut self,
        flow_status: FlowStatus,
        block_size: u8,
        st_min: StMin,
    ) -> Result<(), IsoTpError> {
        let frame = IsoTpFrame::FlowControl {
            flow_status,
            block_size,
            st_min,
        };

        let msg = self.create_tx_message(frame.encode())?;
        self.backend
            .send_message_async(&msg)
            .await
            .map_err(IsoTpError::BackendError)?;

        Ok(())
    }

    /// Receive a single CAN frame and decode as ISO-TP.
    async fn receive_frame(&mut self) -> Result<IsoTpFrame, IsoTpError> {
        loop {
            // Use a short timeout to allow the outer timeout to work
            let poll_timeout = std::time::Duration::from_millis(10);
            let msg = self
                .backend
                .receive_message_async(Some(poll_timeout))
                .await
                .map_err(IsoTpError::BackendError)?;

            // Check if we got a message
            if let Some(msg) = msg {
                // Check if this is our RX ID
                if msg.id().raw() == self.config.rx_id {
                    // Strip address byte for Extended/Mixed addressing
                    let frame_data = self.strip_address_byte(msg.data())?;
                    return IsoTpFrame::decode(frame_data);
                }
            }
            // No message or wrong ID, yield and continue
            tokio::task::yield_now().await;
        }
    }
}

impl<B: CanBackendAsync> std::fmt::Debug for IsoTpChannel<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IsoTpChannel")
            .field("config", &self.config)
            .field("state", &self.state)
            .field("is_fd", &self.is_fd)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_direction() {
        assert_eq!(TransferDirection::Receive, TransferDirection::Receive);
        assert_ne!(TransferDirection::Receive, TransferDirection::Send);
    }
}
