//! Message recorder for tracking sent messages.
//!
//! This module provides the `MessageRecorder` for recording all messages sent through
//! the mock backend, enabling verification in tests.

use canlink_hal::CanMessage;
use std::sync::{Arc, Mutex};

/// Message recorder.
///
/// Records all messages sent through the mock backend for later verification.
/// Thread-safe and can be shared across multiple threads.
///
/// # Examples
///
/// ```
/// use canlink_mock::MessageRecorder;
/// use canlink_hal::CanMessage;
///
/// let recorder = MessageRecorder::new();
/// let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
/// recorder.record(msg.clone());
///
/// let recorded = recorder.get_messages();
/// assert_eq!(recorded.len(), 1);
/// assert_eq!(recorded[0].id(), msg.id());
/// ```
#[derive(Debug, Clone)]
pub struct MessageRecorder {
    messages: Arc<Mutex<Vec<CanMessage>>>,
    max_messages: usize,
}

impl MessageRecorder {
    /// Create a new message recorder.
    ///
    /// # Arguments
    ///
    /// * `max_messages` - Maximum number of messages to record (0 = unlimited)
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MessageRecorder;
    ///
    /// let recorder = MessageRecorder::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Create a new message recorder with a maximum capacity.
    ///
    /// When the capacity is reached, the oldest messages are discarded.
    ///
    /// # Arguments
    ///
    /// * `max_messages` - Maximum number of messages to record (0 = unlimited)
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MessageRecorder;
    ///
    /// let recorder = MessageRecorder::with_capacity(100);
    /// ```
    #[must_use]
    pub fn with_capacity(max_messages: usize) -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
            max_messages,
        }
    }

    /// Record a message.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to record
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MessageRecorder;
    /// use canlink_hal::CanMessage;
    ///
    /// let recorder = MessageRecorder::new();
    /// let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    /// recorder.record(msg);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned (should never happen in normal operation).
    pub fn record(&self, message: CanMessage) {
        let mut messages = self.messages.lock().unwrap();

        // If we have a capacity limit and we're at it, remove the oldest message
        if self.max_messages > 0 && messages.len() >= self.max_messages {
            messages.remove(0);
        }

        messages.push(message);
    }

    /// Get all recorded messages.
    ///
    /// Returns a copy of all recorded messages.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MessageRecorder;
    /// use canlink_hal::CanMessage;
    ///
    /// let recorder = MessageRecorder::new();
    /// let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    /// recorder.record(msg.clone());
    ///
    /// let recorded = recorder.get_messages();
    /// assert_eq!(recorded.len(), 1);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned (should never happen in normal operation).
    #[must_use]
    pub fn get_messages(&self) -> Vec<CanMessage> {
        let messages = self.messages.lock().unwrap();
        messages.clone()
    }

    /// Get the number of recorded messages.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MessageRecorder;
    /// use canlink_hal::CanMessage;
    ///
    /// let recorder = MessageRecorder::new();
    /// assert_eq!(recorder.count(), 0);
    ///
    /// let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    /// recorder.record(msg);
    /// assert_eq!(recorder.count(), 1);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned (should never happen in normal operation).
    #[must_use]
    pub fn count(&self) -> usize {
        let messages = self.messages.lock().unwrap();
        messages.len()
    }

    /// Clear all recorded messages.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MessageRecorder;
    /// use canlink_hal::CanMessage;
    ///
    /// let recorder = MessageRecorder::new();
    /// let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    /// recorder.record(msg);
    /// assert_eq!(recorder.count(), 1);
    ///
    /// recorder.clear();
    /// assert_eq!(recorder.count(), 0);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned (should never happen in normal operation).
    pub fn clear(&self) {
        let mut messages = self.messages.lock().unwrap();
        messages.clear();
    }

    /// Check if a message with the given ID was recorded.
    ///
    /// # Arguments
    ///
    /// * `id` - The CAN ID to search for
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MessageRecorder;
    /// use canlink_hal::{CanMessage, CanId};
    ///
    /// let recorder = MessageRecorder::new();
    /// let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    /// recorder.record(msg);
    ///
    /// assert!(recorder.contains_id(&CanId::Standard(0x123)));
    /// assert!(!recorder.contains_id(&CanId::Standard(0x456)));
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned (should never happen in normal operation).
    #[must_use]
    pub fn contains_id(&self, id: &canlink_hal::CanId) -> bool {
        let messages = self.messages.lock().unwrap();
        messages.iter().any(|msg| msg.id() == *id)
    }

    /// Get messages with a specific ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The CAN ID to filter by
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MessageRecorder;
    /// use canlink_hal::{CanMessage, CanId};
    ///
    /// let recorder = MessageRecorder::new();
    /// recorder.record(CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap());
    /// recorder.record(CanMessage::new_standard(0x456, &[4, 5, 6]).unwrap());
    /// recorder.record(CanMessage::new_standard(0x123, &[7, 8, 9]).unwrap());
    ///
    /// let filtered = recorder.get_messages_by_id(&CanId::Standard(0x123));
    /// assert_eq!(filtered.len(), 2);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned (should never happen in normal operation).
    #[must_use]
    pub fn get_messages_by_id(&self, id: &canlink_hal::CanId) -> Vec<CanMessage> {
        let messages = self.messages.lock().unwrap();
        messages
            .iter()
            .filter(|msg| msg.id() == *id)
            .cloned()
            .collect()
    }
}

impl Default for MessageRecorder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use canlink_hal::CanId;

    #[test]
    fn test_new_recorder() {
        let recorder = MessageRecorder::new();
        assert_eq!(recorder.count(), 0);
    }

    #[test]
    fn test_record_message() {
        let recorder = MessageRecorder::new();
        let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
        recorder.record(msg);
        assert_eq!(recorder.count(), 1);
    }

    #[test]
    fn test_get_messages() {
        let recorder = MessageRecorder::new();
        let msg1 = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
        let msg2 = CanMessage::new_standard(0x456, &[4, 5, 6]).unwrap();

        recorder.record(msg1.clone());
        recorder.record(msg2.clone());

        let messages = recorder.get_messages();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].id(), msg1.id());
        assert_eq!(messages[1].id(), msg2.id());
    }

    #[test]
    fn test_clear() {
        let recorder = MessageRecorder::new();
        let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
        recorder.record(msg);
        assert_eq!(recorder.count(), 1);

        recorder.clear();
        assert_eq!(recorder.count(), 0);
    }

    #[test]
    fn test_contains_id() {
        let recorder = MessageRecorder::new();
        let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
        recorder.record(msg);

        assert!(recorder.contains_id(&CanId::Standard(0x123)));
        assert!(!recorder.contains_id(&CanId::Standard(0x456)));
    }

    #[test]
    fn test_get_messages_by_id() {
        let recorder = MessageRecorder::new();
        recorder.record(CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap());
        recorder.record(CanMessage::new_standard(0x456, &[4, 5, 6]).unwrap());
        recorder.record(CanMessage::new_standard(0x123, &[7, 8, 9]).unwrap());

        let filtered = recorder.get_messages_by_id(&CanId::Standard(0x123));
        assert_eq!(filtered.len(), 2);

        let filtered = recorder.get_messages_by_id(&CanId::Standard(0x456));
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_with_capacity() {
        let recorder = MessageRecorder::with_capacity(2);
        recorder.record(CanMessage::new_standard(0x111, &[1]).unwrap());
        recorder.record(CanMessage::new_standard(0x222, &[2]).unwrap());
        recorder.record(CanMessage::new_standard(0x333, &[3]).unwrap());

        // Should only keep the last 2 messages
        assert_eq!(recorder.count(), 2);
        let messages = recorder.get_messages();
        assert_eq!(messages[0].id(), CanId::Standard(0x222));
        assert_eq!(messages[1].id(), CanId::Standard(0x333));
    }
}
