//! `MessageFilter` trait definition (FR-005)
//!
//! Defines the interface for message filtering.

use crate::message::CanMessage;

/// Message filter trait
///
/// All filter implementations must implement this trait.
/// Filters can be either hardware-accelerated or software-based.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` to allow use across threads.
///
/// # Example
///
/// ```rust,ignore
/// use canlink_hal::filter::MessageFilter;
/// use canlink_hal::message::CanMessage;
///
/// struct MyFilter {
///     target_id: u32,
/// }
///
/// impl MessageFilter for MyFilter {
///     fn matches(&self, message: &CanMessage) -> bool {
///         message.id() == self.target_id
///     }
/// }
/// ```
pub trait MessageFilter: Send + Sync {
    /// Check if a message matches this filter
    ///
    /// Returns `true` if the message should be accepted.
    fn matches(&self, message: &CanMessage) -> bool;

    /// Get the filter priority
    ///
    /// Higher priority filters are evaluated first.
    /// Default is 0 (lowest priority).
    fn priority(&self) -> u32 {
        0
    }

    /// Check if this is a hardware filter
    ///
    /// Hardware filters are executed by the CAN controller,
    /// reducing CPU load. Returns `false` by default.
    fn is_hardware(&self) -> bool {
        false
    }
}
