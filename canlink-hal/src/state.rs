//! Backend state management.
//!
//! This module defines the lifecycle states of a backend.

/// Backend lifecycle state.
///
/// Represents the current state of a backend in its lifecycle.
/// Backends transition through these states during initialization,
/// operation, and shutdown.
///
/// # State Transitions
///
/// ```text
/// Uninitialized -> Initializing -> Ready -> Closing -> Closed
///                       ↓            ↓
///                     Error ←--------┘
/// ```
///
/// # Examples
///
/// ```
/// use canlink_hal::BackendState;
///
/// let state = BackendState::Uninitialized;
/// assert!(!state.is_ready());
///
/// let state = BackendState::Ready;
/// assert!(state.is_ready());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BackendState {
    /// Backend has not been initialized
    #[default]
    Uninitialized,

    /// Backend is currently initializing
    Initializing,

    /// Backend is ready for operations
    Ready,

    /// Backend is closing
    Closing,

    /// Backend has been closed
    Closed,

    /// Backend encountered an error
    Error,
}

impl BackendState {
    /// Check if the backend is ready for operations.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::BackendState;
    ///
    /// assert!(BackendState::Ready.is_ready());
    /// assert!(!BackendState::Uninitialized.is_ready());
    /// ```
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }

    /// Check if the backend is in an error state.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::BackendState;
    ///
    /// assert!(BackendState::Error.is_error());
    /// assert!(!BackendState::Ready.is_error());
    /// ```
    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self, Self::Error)
    }

    /// Check if the backend is closed.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::BackendState;
    ///
    /// assert!(BackendState::Closed.is_closed());
    /// assert!(!BackendState::Ready.is_closed());
    /// ```
    #[must_use]
    pub const fn is_closed(&self) -> bool {
        matches!(self, Self::Closed)
    }

    /// Check if the backend can accept operations.
    ///
    /// Returns true only if the backend is in the Ready state.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::BackendState;
    ///
    /// assert!(BackendState::Ready.can_operate());
    /// assert!(!BackendState::Initializing.can_operate());
    /// assert!(!BackendState::Error.can_operate());
    /// ```
    #[must_use]
    pub const fn can_operate(&self) -> bool {
        matches!(self, Self::Ready)
    }

    /// Get a human-readable description of the state.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::BackendState;
    ///
    /// assert_eq!(BackendState::Ready.description(), "Ready");
    /// assert_eq!(BackendState::Error.description(), "Error");
    /// ```
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::Uninitialized => "Uninitialized",
            Self::Initializing => "Initializing",
            Self::Ready => "Ready",
            Self::Closing => "Closing",
            Self::Closed => "Closed",
            Self::Error => "Error",
        }
    }
}

impl std::fmt::Display for BackendState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_checks() {
        assert!(BackendState::Ready.is_ready());
        assert!(!BackendState::Uninitialized.is_ready());

        assert!(BackendState::Error.is_error());
        assert!(!BackendState::Ready.is_error());

        assert!(BackendState::Closed.is_closed());
        assert!(!BackendState::Ready.is_closed());
    }

    #[test]
    fn test_can_operate() {
        assert!(BackendState::Ready.can_operate());
        assert!(!BackendState::Uninitialized.can_operate());
        assert!(!BackendState::Initializing.can_operate());
        assert!(!BackendState::Closing.can_operate());
        assert!(!BackendState::Closed.can_operate());
        assert!(!BackendState::Error.can_operate());
    }

    #[test]
    fn test_description() {
        assert_eq!(BackendState::Uninitialized.description(), "Uninitialized");
        assert_eq!(BackendState::Initializing.description(), "Initializing");
        assert_eq!(BackendState::Ready.description(), "Ready");
        assert_eq!(BackendState::Closing.description(), "Closing");
        assert_eq!(BackendState::Closed.description(), "Closed");
        assert_eq!(BackendState::Error.description(), "Error");
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", BackendState::Ready), "Ready");
        assert_eq!(format!("{}", BackendState::Error), "Error");
    }

    #[test]
    fn test_default() {
        assert_eq!(BackendState::default(), BackendState::Uninitialized);
    }
}
