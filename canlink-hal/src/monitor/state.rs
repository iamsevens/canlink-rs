//! Connection state enumeration (FR-010)

/// Connection state
///
/// Represents the current state of the backend connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionState {
    /// Connected and operational
    ///
    /// The backend is working normally and can send/receive messages.
    Connected,

    /// Disconnected
    ///
    /// The backend connection has been lost and needs to be re-initialized.
    #[default]
    Disconnected,

    /// Reconnecting
    ///
    /// The system is attempting to reconnect (only when auto-reconnect is enabled).
    Reconnecting,
}

impl ConnectionState {
    /// Check if messages can be sent in this state
    #[must_use]
    pub fn can_send(&self) -> bool {
        matches!(self, Self::Connected)
    }

    /// Check if messages can be received in this state
    #[must_use]
    pub fn can_receive(&self) -> bool {
        matches!(self, Self::Connected)
    }

    /// Check if the connection is active
    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Connected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_send() {
        assert!(ConnectionState::Connected.can_send());
        assert!(!ConnectionState::Disconnected.can_send());
        assert!(!ConnectionState::Reconnecting.can_send());
    }

    #[test]
    fn test_can_receive() {
        assert!(ConnectionState::Connected.can_receive());
        assert!(!ConnectionState::Disconnected.can_receive());
        assert!(!ConnectionState::Reconnecting.can_receive());
    }
}
