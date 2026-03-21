//! Mock backend implementation.
//!
//! This module provides the `MockBackend` implementation of the `CanBackend` trait
//! for testing purposes.

use crate::{ErrorInjector, MessageRecorder, MockConfig};
use canlink_hal::{
    filter::{FilterChain, IdFilter, MessageFilter, RangeFilter},
    BackendConfig, BackendFactory, BackendState, BackendVersion, CanBackend, CanError, CanMessage,
    CanResult, HardwareCapability,
};
use std::collections::HashSet;

/// Mock CAN backend for testing.
///
/// Provides a simulated CAN hardware backend that records sent messages,
/// returns preset messages, and can simulate various error conditions.
///
/// # Filtering
///
/// The mock backend supports message filtering via `FilterChain`. When filters
/// are configured, `receive_message()` will only return messages that pass
/// the filter chain.
///
/// # Examples
///
/// ```
/// use canlink_mock::MockBackend;
/// use canlink_hal::{CanBackend, BackendConfig, CanMessage};
///
/// let mut backend = MockBackend::new();
/// let config = BackendConfig::new("mock");
/// backend.initialize(&config).unwrap();
/// backend.open_channel(0).unwrap();
///
/// // Send a message
/// let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
/// backend.send_message(&msg).unwrap();
///
/// // Verify it was recorded
/// let recorded = backend.get_recorded_messages();
/// assert_eq!(recorded.len(), 1);
///
/// backend.close().unwrap();
/// ```
pub struct MockBackend {
    state: BackendState,
    config: MockConfig,
    recorder: MessageRecorder,
    open_channels: HashSet<u8>,
    preset_message_index: usize,
    error_injector: ErrorInjector,
    /// Filter chain for receive filtering
    filter_chain: FilterChain,
}

impl MockBackend {
    /// Create a new mock backend with default configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MockBackend;
    ///
    /// let backend = MockBackend::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(MockConfig::default())
    }

    /// Create a new mock backend with custom configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Mock backend configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::{MockBackend, MockConfig};
    ///
    /// let config = MockConfig::can20_only();
    /// let backend = MockBackend::with_config(config);
    /// ```
    #[must_use]
    pub fn with_config(config: MockConfig) -> Self {
        let recorder = MessageRecorder::with_capacity(config.max_recorded_messages);
        let filter_chain = FilterChain::new(config.filter_count as usize);
        Self {
            state: BackendState::Uninitialized,
            config,
            recorder,
            open_channels: HashSet::new(),
            preset_message_index: 0,
            error_injector: ErrorInjector::new(),
            filter_chain,
        }
    }

    /// Get all recorded messages.
    ///
    /// Returns a copy of all messages that were sent through this backend.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MockBackend;
    /// use canlink_hal::{CanBackend, BackendConfig, CanMessage};
    ///
    /// let mut backend = MockBackend::new();
    /// backend.initialize(&BackendConfig::new("mock")).unwrap();
    /// backend.open_channel(0).unwrap();
    ///
    /// let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    /// backend.send_message(&msg).unwrap();
    ///
    /// let recorded = backend.get_recorded_messages();
    /// assert_eq!(recorded.len(), 1);
    /// ```
    #[must_use]
    pub fn get_recorded_messages(&self) -> Vec<CanMessage> {
        self.recorder.get_messages()
    }

    /// Clear all recorded messages.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MockBackend;
    /// use canlink_hal::{CanBackend, BackendConfig, CanMessage};
    ///
    /// let mut backend = MockBackend::new();
    /// backend.initialize(&BackendConfig::new("mock")).unwrap();
    /// backend.open_channel(0).unwrap();
    ///
    /// let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    /// backend.send_message(&msg).unwrap();
    /// assert_eq!(backend.get_recorded_messages().len(), 1);
    ///
    /// backend.clear_recorded_messages();
    /// assert_eq!(backend.get_recorded_messages().len(), 0);
    /// ```
    pub fn clear_recorded_messages(&mut self) {
        self.recorder.clear();
    }

    /// Get the current backend state.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MockBackend;
    /// use canlink_hal::{BackendState, CanBackend, BackendConfig};
    ///
    /// let mut backend = MockBackend::new();
    /// assert_eq!(backend.get_state(), BackendState::Uninitialized);
    ///
    /// backend.initialize(&BackendConfig::new("mock")).unwrap();
    /// assert_eq!(backend.get_state(), BackendState::Ready);
    /// ```
    #[must_use]
    pub fn get_state(&self) -> BackendState {
        self.state
    }

    /// Get the mock configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::{MockBackend, MockConfig};
    ///
    /// let config = MockConfig::can20_only();
    /// let backend = MockBackend::with_config(config.clone());
    /// assert_eq!(backend.get_config().channel_count, config.channel_count);
    /// ```
    #[must_use]
    pub fn get_config(&self) -> &MockConfig {
        &self.config
    }

    /// Get mutable access to the error injector.
    ///
    /// Allows configuring error injection scenarios for testing.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MockBackend;
    /// use canlink_hal::CanError;
    ///
    /// let mut backend = MockBackend::new();
    /// backend.error_injector_mut().inject_send_error(CanError::SendFailed {
    ///     reason: "Test error".to_string(),
    /// });
    /// ```
    pub fn error_injector_mut(&mut self) -> &mut ErrorInjector {
        &mut self.error_injector
    }

    /// Get read-only access to the error injector.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MockBackend;
    ///
    /// let backend = MockBackend::new();
    /// let count = backend.error_injector().injection_count();
    /// assert_eq!(count, 0);
    /// ```
    #[must_use]
    pub fn error_injector(&self) -> &ErrorInjector {
        &self.error_injector
    }

    /// Verify that a message with the given ID was sent.
    ///
    /// # Arguments
    ///
    /// * `id` - The CAN ID to search for
    ///
    /// # Returns
    ///
    /// `true` if at least one message with the given ID was recorded.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MockBackend;
    /// use canlink_hal::{CanBackend, BackendConfig, CanMessage, CanId};
    ///
    /// let mut backend = MockBackend::new();
    /// backend.initialize(&BackendConfig::new("mock")).unwrap();
    /// backend.open_channel(0).unwrap();
    ///
    /// let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    /// backend.send_message(&msg).unwrap();
    ///
    /// assert!(backend.verify_message_sent(CanId::Standard(0x123)));
    /// assert!(!backend.verify_message_sent(CanId::Standard(0x456)));
    /// ```
    #[must_use]
    pub fn verify_message_sent(&self, id: canlink_hal::CanId) -> bool {
        self.recorder.contains_id(&id)
    }

    /// Get all messages sent with a specific CAN ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The CAN ID to filter by
    ///
    /// # Returns
    ///
    /// A vector of all messages with the given ID.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MockBackend;
    /// use canlink_hal::{CanBackend, BackendConfig, CanMessage, CanId};
    ///
    /// let mut backend = MockBackend::new();
    /// backend.initialize(&BackendConfig::new("mock")).unwrap();
    /// backend.open_channel(0).unwrap();
    ///
    /// backend.send_message(&CanMessage::new_standard(0x123, &[1]).unwrap()).unwrap();
    /// backend.send_message(&CanMessage::new_standard(0x123, &[2]).unwrap()).unwrap();
    /// backend.send_message(&CanMessage::new_standard(0x456, &[3]).unwrap()).unwrap();
    ///
    /// let messages = backend.get_messages_by_id(CanId::Standard(0x123));
    /// assert_eq!(messages.len(), 2);
    /// ```
    #[must_use]
    pub fn get_messages_by_id(&self, id: canlink_hal::CanId) -> Vec<CanMessage> {
        self.recorder.get_messages_by_id(&id)
    }

    /// Verify that exactly N messages were sent.
    ///
    /// # Arguments
    ///
    /// * `count` - Expected number of messages
    ///
    /// # Returns
    ///
    /// `true` if exactly `count` messages were recorded.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MockBackend;
    /// use canlink_hal::{CanBackend, BackendConfig, CanMessage};
    ///
    /// let mut backend = MockBackend::new();
    /// backend.initialize(&BackendConfig::new("mock")).unwrap();
    /// backend.open_channel(0).unwrap();
    ///
    /// backend.send_message(&CanMessage::new_standard(0x123, &[1]).unwrap()).unwrap();
    /// backend.send_message(&CanMessage::new_standard(0x456, &[2]).unwrap()).unwrap();
    ///
    /// assert!(backend.verify_message_count(2));
    /// assert!(!backend.verify_message_count(3));
    /// ```
    #[must_use]
    pub fn verify_message_count(&self, count: usize) -> bool {
        self.recorder.count() == count
    }

    // ========================================================================
    // Filter Management (FR-006)
    // ========================================================================

    /// Add a filter to the receive filter chain.
    ///
    /// Messages received via `receive_message()` will be filtered according
    /// to the filter chain. An empty chain passes all messages.
    ///
    /// # Arguments
    ///
    /// * `filter` - The filter to add
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MockBackend;
    /// use canlink_hal::filter::IdFilter;
    ///
    /// let mut backend = MockBackend::new();
    /// backend.add_filter(Box::new(IdFilter::new(0x123)));
    /// ```
    pub fn add_filter(&mut self, filter: Box<dyn MessageFilter>) {
        self.filter_chain.add_filter(filter);
    }

    /// Add an ID filter for a specific CAN ID.
    ///
    /// Convenience method to add a filter for a single standard CAN ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The CAN ID to filter (standard frame)
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MockBackend;
    ///
    /// let mut backend = MockBackend::new();
    /// backend.add_id_filter(0x123);
    /// backend.add_id_filter(0x456);
    /// ```
    pub fn add_id_filter(&mut self, id: u32) {
        self.filter_chain.add_filter(Box::new(IdFilter::new(id)));
    }

    /// Add a range filter for a range of CAN IDs.
    ///
    /// Convenience method to add a filter for a range of standard CAN IDs.
    ///
    /// # Arguments
    ///
    /// * `start_id` - Start of the ID range (inclusive)
    /// * `end_id` - End of the ID range (inclusive)
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MockBackend;
    ///
    /// let mut backend = MockBackend::new();
    /// backend.add_range_filter(0x100, 0x1FF);
    /// ```
    pub fn add_range_filter(&mut self, start_id: u32, end_id: u32) {
        self.filter_chain
            .add_filter(Box::new(RangeFilter::new(start_id, end_id)));
    }

    /// Clear all filters from the filter chain.
    ///
    /// After clearing, all messages will pass through (no filtering).
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MockBackend;
    ///
    /// let mut backend = MockBackend::new();
    /// backend.add_id_filter(0x123);
    /// assert_eq!(backend.filter_count(), 1);
    ///
    /// backend.clear_filters();
    /// assert_eq!(backend.filter_count(), 0);
    /// ```
    pub fn clear_filters(&mut self) {
        self.filter_chain.clear();
    }

    /// Get the number of filters in the chain.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MockBackend;
    ///
    /// let mut backend = MockBackend::new();
    /// assert_eq!(backend.filter_count(), 0);
    ///
    /// backend.add_id_filter(0x123);
    /// assert_eq!(backend.filter_count(), 1);
    /// ```
    #[must_use]
    pub fn filter_count(&self) -> usize {
        self.filter_chain.len()
    }

    /// Get read-only access to the filter chain.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MockBackend;
    ///
    /// let backend = MockBackend::new();
    /// assert!(backend.filter_chain().is_empty());
    /// ```
    #[must_use]
    pub fn filter_chain(&self) -> &FilterChain {
        &self.filter_chain
    }

    /// Get mutable access to the filter chain.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MockBackend;
    /// use canlink_hal::filter::IdFilter;
    ///
    /// let mut backend = MockBackend::new();
    /// backend.filter_chain_mut().add_filter(Box::new(IdFilter::new(0x123)));
    /// ```
    pub fn filter_chain_mut(&mut self) -> &mut FilterChain {
        &mut self.filter_chain
    }

    // ========== Connection Simulation (FR-010 Testing) ==========

    /// Simulate a hardware disconnection.
    ///
    /// This method transitions the backend to an error state, simulating
    /// what happens when CAN hardware is physically disconnected. After
    /// calling this method:
    ///
    /// - `send_message()` will return `CanError::SendFailed`
    /// - `receive_message()` will return `CanError::ReceiveFailed`
    /// - `get_state()` will return `BackendState::Error`
    ///
    /// Use `simulate_reconnect()` to restore normal operation.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MockBackend;
    /// use canlink_hal::{CanBackend, BackendConfig, BackendState, CanMessage};
    ///
    /// let mut backend = MockBackend::new();
    /// backend.initialize(&BackendConfig::new("mock")).unwrap();
    /// backend.open_channel(0).unwrap();
    ///
    /// // Simulate disconnection
    /// backend.simulate_disconnect();
    /// assert_eq!(backend.get_state(), BackendState::Error);
    ///
    /// // Operations now fail
    /// let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    /// assert!(backend.send_message(&msg).is_err());
    /// ```
    pub fn simulate_disconnect(&mut self) {
        self.state = BackendState::Error;
    }

    /// Simulate a hardware reconnection.
    ///
    /// This method restores the backend to the Ready state after a simulated
    /// disconnection. It simulates successful hardware reconnection.
    ///
    /// # Note
    ///
    /// This only works if the backend was previously in the Error state
    /// (from `simulate_disconnect()`). If the backend is in another state,
    /// this method has no effect.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MockBackend;
    /// use canlink_hal::{CanBackend, BackendConfig, BackendState, CanMessage};
    ///
    /// let mut backend = MockBackend::new();
    /// backend.initialize(&BackendConfig::new("mock")).unwrap();
    /// backend.open_channel(0).unwrap();
    ///
    /// // Simulate disconnect then reconnect
    /// backend.simulate_disconnect();
    /// assert_eq!(backend.get_state(), BackendState::Error);
    ///
    /// backend.simulate_reconnect();
    /// assert_eq!(backend.get_state(), BackendState::Ready);
    ///
    /// // Operations work again
    /// let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    /// assert!(backend.send_message(&msg).is_ok());
    /// ```
    pub fn simulate_reconnect(&mut self) {
        if self.state == BackendState::Error {
            self.state = BackendState::Ready;
        }
    }

    /// Check if the backend is in a simulated disconnected state.
    ///
    /// # Returns
    ///
    /// `true` if the backend is in the Error state (disconnected),
    /// `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MockBackend;
    /// use canlink_hal::{CanBackend, BackendConfig};
    ///
    /// let mut backend = MockBackend::new();
    /// backend.initialize(&BackendConfig::new("mock")).unwrap();
    ///
    /// assert!(!backend.is_disconnected());
    ///
    /// backend.simulate_disconnect();
    /// assert!(backend.is_disconnected());
    /// ```
    #[must_use]
    pub fn is_disconnected(&self) -> bool {
        self.state == BackendState::Error
    }
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CanBackend for MockBackend {
    fn initialize(&mut self, config: &BackendConfig) -> CanResult<()> {
        // Check if already initialized (allow re-initialization after close)
        if self.state != BackendState::Uninitialized && self.state != BackendState::Closed {
            return Err(CanError::InvalidState {
                expected: "Uninitialized or Closed".to_string(),
                current: format!("{:?}", self.state),
            });
        }

        // Check error injector first
        if let Some(error) = self.error_injector.should_fail_init() {
            return Err(error);
        }

        // Simulate initialization failure if configured
        if self.config.fail_initialization {
            return Err(CanError::InitializationFailed {
                reason: "Mock backend configured to fail initialization".to_string(),
            });
        }

        // Parse mock-specific configuration from backend config if present
        if let Some(params) = config.parameters.get("mock") {
            // Try to deserialize mock config from parameters
            // Convert TOML value to JSON string and back to deserialize
            if let Ok(json_str) = serde_json::to_string(params) {
                if let Ok(mock_config) = serde_json::from_str::<MockConfig>(&json_str) {
                    self.config = mock_config;
                    self.recorder =
                        MessageRecorder::with_capacity(self.config.max_recorded_messages);
                }
            }
        }

        self.state = BackendState::Ready;
        Ok(())
    }

    fn close(&mut self) -> CanResult<()> {
        // Idempotency: if already closed, return success
        if self.state == BackendState::Closed {
            return Ok(());
        }

        // Check if in ready state
        if self.state != BackendState::Ready {
            return Err(CanError::InvalidState {
                expected: "Ready or Closed".to_string(),
                current: format!("{:?}", self.state),
            });
        }

        // Close all open channels
        self.open_channels.clear();

        // Clear message recorder, preset message index, and filters
        self.recorder.clear();
        self.preset_message_index = 0;
        self.filter_chain.clear();

        self.state = BackendState::Closed;
        Ok(())
    }

    fn get_capability(&self) -> CanResult<HardwareCapability> {
        Ok(self.config.to_capability())
    }

    fn send_message(&mut self, message: &CanMessage) -> CanResult<()> {
        // Check for simulated disconnection
        if self.state == BackendState::Error {
            return Err(CanError::SendFailed {
                reason: "Hardware disconnected".to_string(),
            });
        }

        // Check state
        if self.state != BackendState::Ready {
            return Err(CanError::InvalidState {
                expected: "Ready".to_string(),
                current: format!("{:?}", self.state),
            });
        }

        // Check if any channel is open
        if self.open_channels.is_empty() {
            return Err(CanError::ChannelNotOpen { channel: 0 });
        }

        // Check error injector first
        if let Some(error) = self.error_injector.should_fail_send() {
            return Err(error);
        }

        // Simulate send failure if configured
        if self.config.fail_send {
            return Err(CanError::SendFailed {
                reason: "Mock backend configured to fail send".to_string(),
            });
        }

        // Check CAN-FD support
        if message.is_fd() && !self.config.supports_canfd {
            return Err(CanError::UnsupportedFeature {
                feature: "CAN-FD".to_string(),
            });
        }

        // Record the message
        self.recorder.record(message.clone());
        Ok(())
    }

    fn receive_message(&mut self) -> CanResult<Option<CanMessage>> {
        // Check for simulated disconnection
        if self.state == BackendState::Error {
            return Err(CanError::ReceiveFailed {
                reason: "Hardware disconnected".to_string(),
            });
        }

        // Check state
        if self.state != BackendState::Ready {
            return Err(CanError::InvalidState {
                expected: "Ready".to_string(),
                current: format!("{:?}", self.state),
            });
        }

        // Check if any channel is open
        if self.open_channels.is_empty() {
            return Err(CanError::ChannelNotOpen { channel: 0 });
        }

        // Check error injector first
        if let Some(error) = self.error_injector.should_fail_receive() {
            return Err(error);
        }

        // Simulate receive failure if configured
        if self.config.fail_receive {
            return Err(CanError::ReceiveFailed {
                reason: "Mock backend configured to fail receive".to_string(),
            });
        }

        // Return preset messages in order, applying filter chain
        while self.preset_message_index < self.config.preset_messages.len() {
            let message = self.config.preset_messages[self.preset_message_index].clone();
            self.preset_message_index += 1;

            // Apply filter chain - if message passes, return it
            if self.filter_chain.matches(&message) {
                return Ok(Some(message));
            }
            // Message filtered out, continue to next preset message
        }

        Ok(None)
    }

    fn open_channel(&mut self, channel: u8) -> CanResult<()> {
        // Check state
        if self.state != BackendState::Ready {
            return Err(CanError::InvalidState {
                expected: "Ready".to_string(),
                current: format!("{:?}", self.state),
            });
        }

        // Check error injector first
        if let Some(error) = self.error_injector.should_fail_open_channel() {
            return Err(error);
        }

        // Check if channel exists
        if channel >= self.config.channel_count {
            return Err(CanError::ChannelNotFound {
                channel,
                max: self.config.channel_count - 1,
            });
        }

        // Check if already open
        if self.open_channels.contains(&channel) {
            return Err(CanError::ChannelAlreadyOpen { channel });
        }

        self.open_channels.insert(channel);
        Ok(())
    }

    fn close_channel(&mut self, channel: u8) -> CanResult<()> {
        // Check state
        if self.state != BackendState::Ready {
            return Err(CanError::InvalidState {
                expected: "Ready".to_string(),
                current: format!("{:?}", self.state),
            });
        }

        // Check error injector first
        if let Some(error) = self.error_injector.should_fail_close_channel() {
            return Err(error);
        }

        // Check if channel exists
        if channel >= self.config.channel_count {
            return Err(CanError::ChannelNotFound {
                channel,
                max: self.config.channel_count - 1,
            });
        }

        // Check if open
        if !self.open_channels.contains(&channel) {
            return Err(CanError::ChannelNotOpen { channel });
        }

        self.open_channels.remove(&channel);
        Ok(())
    }

    fn version(&self) -> BackendVersion {
        BackendVersion::new(0, 1, 0)
    }

    fn name(&self) -> &'static str {
        "mock"
    }
}

/// Mock backend factory.
///
/// Creates instances of `MockBackend` with configuration from `BackendConfig`.
///
/// # Examples
///
/// ```
/// use canlink_mock::MockBackendFactory;
/// use canlink_hal::{BackendFactory, BackendConfig};
///
/// let factory = MockBackendFactory::new();
/// let config = BackendConfig::new("mock");
/// let backend = factory.create(&config).unwrap();
/// ```
pub struct MockBackendFactory {
    default_config: MockConfig,
}

impl MockBackendFactory {
    /// Create a new mock backend factory with default configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MockBackendFactory;
    ///
    /// let factory = MockBackendFactory::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            default_config: MockConfig::default(),
        }
    }

    /// Create a new mock backend factory with custom default configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Default configuration for created backends
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::{MockBackendFactory, MockConfig};
    ///
    /// let config = MockConfig::can20_only();
    /// let factory = MockBackendFactory::with_config(config);
    /// ```
    #[must_use]
    pub fn with_config(config: MockConfig) -> Self {
        Self {
            default_config: config,
        }
    }
}

impl Default for MockBackendFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl BackendFactory for MockBackendFactory {
    fn create(&self, _config: &BackendConfig) -> CanResult<Box<dyn CanBackend>> {
        Ok(Box::new(MockBackend::with_config(
            self.default_config.clone(),
        )))
    }

    fn name(&self) -> &'static str {
        "mock"
    }

    fn version(&self) -> BackendVersion {
        BackendVersion::new(0, 1, 0)
    }
}

// ============================================================================
// Async Implementation (feature-gated)
// ============================================================================

#[cfg(feature = "async")]
use canlink_hal::CanBackendAsync;

#[cfg(feature = "async")]
use std::time::Duration;

/// Async implementation for `MockBackend`.
///
/// This implementation wraps the synchronous methods and adds async-specific
/// functionality like timeout support for receive operations.
#[cfg(feature = "async")]
impl CanBackendAsync for MockBackend {
    async fn send_message_async(&mut self, message: &CanMessage) -> CanResult<()> {
        // For mock backend, async send is the same as sync send
        // In a real hardware backend, this would use async I/O
        self.send_message(message)
    }

    async fn receive_message_async(
        &mut self,
        timeout: Option<Duration>,
    ) -> CanResult<Option<CanMessage>> {
        match timeout {
            None => {
                // Non-blocking: same as sync version
                self.receive_message()
            }
            Some(duration) => {
                // With timeout: poll until message available or timeout
                let start = std::time::Instant::now();
                let poll_interval = Duration::from_millis(1);

                loop {
                    // Try to receive
                    if let Some(msg) = self.receive_message()? {
                        return Ok(Some(msg));
                    }
                    // Check timeout
                    if start.elapsed() >= duration {
                        return Ok(None);
                    }
                    // Sleep briefly before next poll
                    #[cfg(feature = "async-tokio")]
                    tokio::time::sleep(poll_interval).await;
                    #[cfg(all(feature = "async-async-std", not(feature = "async-tokio")))]
                    async_std::task::sleep(poll_interval).await;
                    #[cfg(all(
                        feature = "async",
                        not(feature = "async-tokio"),
                        not(feature = "async-async-std")
                    ))]
                    std::thread::sleep(poll_interval);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use canlink_hal::CanId;

    #[test]
    fn test_new_backend() {
        let backend = MockBackend::new();
        assert_eq!(backend.get_state(), BackendState::Uninitialized);
        assert_eq!(backend.name(), "mock");
    }

    #[test]
    fn test_initialize() {
        let mut backend = MockBackend::new();
        let config = BackendConfig::new("mock");
        assert!(backend.initialize(&config).is_ok());
        assert_eq!(backend.get_state(), BackendState::Ready);
    }

    #[test]
    fn test_initialize_already_initialized() {
        let mut backend = MockBackend::new();
        let config = BackendConfig::new("mock");
        backend.initialize(&config).unwrap();
        assert!(backend.initialize(&config).is_err());
    }

    #[test]
    fn test_initialize_failure() {
        let config = MockConfig {
            fail_initialization: true,
            ..Default::default()
        };
        let mut backend = MockBackend::with_config(config);
        let backend_config = BackendConfig::new("mock");
        assert!(backend.initialize(&backend_config).is_err());
    }

    #[test]
    fn test_close() {
        let mut backend = MockBackend::new();
        let config = BackendConfig::new("mock");
        backend.initialize(&config).unwrap();
        assert!(backend.close().is_ok());
        assert_eq!(backend.get_state(), BackendState::Closed);
    }

    #[test]
    fn test_get_capability() {
        let backend = MockBackend::new();
        let capability = backend.get_capability().unwrap();
        assert_eq!(capability.channel_count, 2);
        assert!(capability.supports_canfd);
    }

    #[test]
    fn test_send_message() {
        let mut backend = MockBackend::new();
        let config = BackendConfig::new("mock");
        backend.initialize(&config).unwrap();
        backend.open_channel(0).unwrap();

        let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
        assert!(backend.send_message(&msg).is_ok());

        let recorded = backend.get_recorded_messages();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].id(), CanId::Standard(0x123));
    }

    #[test]
    fn test_send_message_no_channel_open() {
        let mut backend = MockBackend::new();
        let config = BackendConfig::new("mock");
        backend.initialize(&config).unwrap();

        let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
        assert!(backend.send_message(&msg).is_err());
    }

    #[test]
    fn test_send_canfd_without_support() {
        let config = MockConfig {
            supports_canfd: false,
            ..Default::default()
        };
        let mut backend = MockBackend::with_config(config);
        let backend_config = BackendConfig::new("mock");
        backend.initialize(&backend_config).unwrap();
        backend.open_channel(0).unwrap();

        let msg = CanMessage::new_fd(CanId::Standard(0x123), &[1, 2, 3]).unwrap();
        assert!(backend.send_message(&msg).is_err());
    }

    #[test]
    fn test_receive_preset_messages() {
        let preset = vec![
            CanMessage::new_standard(0x111, &[1]).unwrap(),
            CanMessage::new_standard(0x222, &[2]).unwrap(),
        ];
        let config = MockConfig::with_preset_messages(preset);
        let mut backend = MockBackend::with_config(config);
        let backend_config = BackendConfig::new("mock");
        backend.initialize(&backend_config).unwrap();
        backend.open_channel(0).unwrap();

        let msg1 = backend.receive_message().unwrap();
        assert!(msg1.is_some());
        assert_eq!(msg1.unwrap().id(), CanId::Standard(0x111));

        let msg2 = backend.receive_message().unwrap();
        assert!(msg2.is_some());
        assert_eq!(msg2.unwrap().id(), CanId::Standard(0x222));

        let msg3 = backend.receive_message().unwrap();
        assert!(msg3.is_none());
    }

    #[test]
    fn test_open_channel() {
        let mut backend = MockBackend::new();
        let config = BackendConfig::new("mock");
        backend.initialize(&config).unwrap();
        assert!(backend.open_channel(0).is_ok());
        assert!(backend.open_channel(1).is_ok());
    }

    #[test]
    fn test_open_invalid_channel() {
        let mut backend = MockBackend::new();
        let config = BackendConfig::new("mock");
        backend.initialize(&config).unwrap();
        assert!(backend.open_channel(99).is_err());
    }

    #[test]
    fn test_open_channel_already_open() {
        let mut backend = MockBackend::new();
        let config = BackendConfig::new("mock");
        backend.initialize(&config).unwrap();
        backend.open_channel(0).unwrap();
        assert!(backend.open_channel(0).is_err());
    }

    #[test]
    fn test_close_channel() {
        let mut backend = MockBackend::new();
        let config = BackendConfig::new("mock");
        backend.initialize(&config).unwrap();
        backend.open_channel(0).unwrap();
        assert!(backend.close_channel(0).is_ok());
    }

    #[test]
    fn test_close_channel_not_open() {
        let mut backend = MockBackend::new();
        let config = BackendConfig::new("mock");
        backend.initialize(&config).unwrap();
        assert!(backend.close_channel(0).is_err());
    }

    #[test]
    fn test_factory_create() {
        let factory = MockBackendFactory::new();
        let config = BackendConfig::new("mock");
        let backend = factory.create(&config);
        assert!(backend.is_ok());
    }

    #[test]
    fn test_factory_name_version() {
        let factory = MockBackendFactory::new();
        assert_eq!(factory.name(), "mock");
        assert_eq!(factory.version().major(), 0);
        assert_eq!(factory.version().minor(), 1);
    }

    #[test]
    fn test_close_idempotent() {
        let mut backend = MockBackend::new();
        let config = BackendConfig::new("mock");
        backend.initialize(&config).unwrap();

        // First close
        assert!(backend.close().is_ok());
        assert_eq!(backend.get_state(), BackendState::Closed);

        // Second close should succeed (idempotency)
        assert!(backend.close().is_ok());
        assert_eq!(backend.get_state(), BackendState::Closed);

        // Third close should also succeed
        assert!(backend.close().is_ok());
    }

    #[test]
    fn test_close_clears_resources() {
        let mut backend = MockBackend::new();
        let config = BackendConfig::new("mock");
        backend.initialize(&config).unwrap();
        backend.open_channel(0).unwrap();

        // Send some messages
        let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
        backend.send_message(&msg).unwrap();
        assert_eq!(backend.get_recorded_messages().len(), 1);

        // Close backend
        backend.close().unwrap();

        // Re-initialize
        backend.initialize(&config).unwrap();

        // Verify resources were cleared
        assert_eq!(backend.get_recorded_messages().len(), 0);
    }

    // ========================================================================
    // Filter Tests
    // ========================================================================

    #[test]
    fn test_filter_chain_empty_passes_all() {
        let preset = vec![
            CanMessage::new_standard(0x111, &[1]).unwrap(),
            CanMessage::new_standard(0x222, &[2]).unwrap(),
        ];
        let config = MockConfig::with_preset_messages(preset);
        let mut backend = MockBackend::with_config(config);
        let backend_config = BackendConfig::new("mock");
        backend.initialize(&backend_config).unwrap();
        backend.open_channel(0).unwrap();

        // No filters - all messages should pass
        assert_eq!(backend.filter_count(), 0);

        let msg1 = backend.receive_message().unwrap();
        assert!(msg1.is_some());
        assert_eq!(msg1.unwrap().id(), CanId::Standard(0x111));

        let msg2 = backend.receive_message().unwrap();
        assert!(msg2.is_some());
        assert_eq!(msg2.unwrap().id(), CanId::Standard(0x222));
    }

    #[test]
    fn test_filter_id_filter() {
        let preset = vec![
            CanMessage::new_standard(0x111, &[1]).unwrap(),
            CanMessage::new_standard(0x222, &[2]).unwrap(),
            CanMessage::new_standard(0x333, &[3]).unwrap(),
        ];
        let config = MockConfig::with_preset_messages(preset);
        let mut backend = MockBackend::with_config(config);
        let backend_config = BackendConfig::new("mock");
        backend.initialize(&backend_config).unwrap();
        backend.open_channel(0).unwrap();

        // Add filter for 0x222 only
        backend.add_id_filter(0x222);
        assert_eq!(backend.filter_count(), 1);

        // Should only receive 0x222
        let msg = backend.receive_message().unwrap();
        assert!(msg.is_some());
        assert_eq!(msg.unwrap().id(), CanId::Standard(0x222));

        // No more messages should pass
        let msg = backend.receive_message().unwrap();
        assert!(msg.is_none());
    }

    #[test]
    fn test_filter_multiple_id_filters() {
        let preset = vec![
            CanMessage::new_standard(0x111, &[1]).unwrap(),
            CanMessage::new_standard(0x222, &[2]).unwrap(),
            CanMessage::new_standard(0x333, &[3]).unwrap(),
            CanMessage::new_standard(0x444, &[4]).unwrap(),
        ];
        let config = MockConfig::with_preset_messages(preset);
        let mut backend = MockBackend::with_config(config);
        let backend_config = BackendConfig::new("mock");
        backend.initialize(&backend_config).unwrap();
        backend.open_channel(0).unwrap();

        // Add filters for 0x111 and 0x333
        backend.add_id_filter(0x111);
        backend.add_id_filter(0x333);
        assert_eq!(backend.filter_count(), 2);

        // Should receive 0x111 and 0x333
        let msg1 = backend.receive_message().unwrap();
        assert!(msg1.is_some());
        assert_eq!(msg1.unwrap().id(), CanId::Standard(0x111));

        let msg2 = backend.receive_message().unwrap();
        assert!(msg2.is_some());
        assert_eq!(msg2.unwrap().id(), CanId::Standard(0x333));

        // No more messages
        let msg3 = backend.receive_message().unwrap();
        assert!(msg3.is_none());
    }

    #[test]
    fn test_filter_range_filter() {
        let preset = vec![
            CanMessage::new_standard(0x100, &[1]).unwrap(),
            CanMessage::new_standard(0x150, &[2]).unwrap(),
            CanMessage::new_standard(0x1FF, &[3]).unwrap(),
            CanMessage::new_standard(0x200, &[4]).unwrap(),
        ];
        let config = MockConfig::with_preset_messages(preset);
        let mut backend = MockBackend::with_config(config);
        let backend_config = BackendConfig::new("mock");
        backend.initialize(&backend_config).unwrap();
        backend.open_channel(0).unwrap();

        // Add range filter for 0x100-0x1FF
        backend.add_range_filter(0x100, 0x1FF);

        // Should receive 0x100, 0x150, 0x1FF but not 0x200
        let msg1 = backend.receive_message().unwrap();
        assert_eq!(msg1.unwrap().id(), CanId::Standard(0x100));

        let msg2 = backend.receive_message().unwrap();
        assert_eq!(msg2.unwrap().id(), CanId::Standard(0x150));

        let msg3 = backend.receive_message().unwrap();
        assert_eq!(msg3.unwrap().id(), CanId::Standard(0x1FF));

        // 0x200 should be filtered out
        let msg4 = backend.receive_message().unwrap();
        assert!(msg4.is_none());
    }

    #[test]
    fn test_filter_clear_filters() {
        let preset = vec![
            CanMessage::new_standard(0x111, &[1]).unwrap(),
            CanMessage::new_standard(0x222, &[2]).unwrap(),
        ];
        let config = MockConfig::with_preset_messages(preset);
        let mut backend = MockBackend::with_config(config);
        let backend_config = BackendConfig::new("mock");
        backend.initialize(&backend_config).unwrap();
        backend.open_channel(0).unwrap();

        // Add filter that blocks all preset messages (0x7FF is valid standard ID)
        backend.add_id_filter(0x7FF);
        assert_eq!(backend.filter_count(), 1);

        // No messages should pass (0x111 and 0x222 don't match 0x7FF)
        let msg = backend.receive_message().unwrap();
        assert!(msg.is_none());

        // Clear filters and reset preset index for new test
        backend.clear_filters();
        assert_eq!(backend.filter_count(), 0);
    }

    #[test]
    fn test_filter_close_clears_filters() {
        let mut backend = MockBackend::new();
        let config = BackendConfig::new("mock");
        backend.initialize(&config).unwrap();

        // Add some filters
        backend.add_id_filter(0x123);
        backend.add_id_filter(0x456);
        assert_eq!(backend.filter_count(), 2);

        // Close backend
        backend.close().unwrap();

        // Re-initialize
        backend.initialize(&config).unwrap();

        // Filters should be cleared
        assert_eq!(backend.filter_count(), 0);
    }

    #[test]
    fn test_filter_chain_access() {
        use canlink_hal::filter::IdFilter;

        let mut backend = MockBackend::new();

        // Test filter_chain() accessor
        assert!(backend.filter_chain().is_empty());

        // Test filter_chain_mut() accessor
        backend
            .filter_chain_mut()
            .add_filter(Box::new(IdFilter::new(0x123)));
        assert_eq!(backend.filter_chain().len(), 1);
    }
}
