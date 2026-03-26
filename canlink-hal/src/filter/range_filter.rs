//! Range filter implementation (FR-006)
//!
//! Provides ID range-based filtering.

use crate::error::FilterError;
use crate::message::CanMessage;

use super::id_filter::{MAX_EXTENDED_ID, MAX_STANDARD_ID};
use super::MessageFilter;

/// Range filter for matching a range of CAN IDs
///
/// Matches any message with an ID between `start_id` and `end_id` (inclusive).
///
/// # Example
///
/// ```rust
/// use canlink_hal::filter::RangeFilter;
///
/// // Match IDs 0x100-0x1FF
/// let filter = RangeFilter::new(0x100, 0x1FF);
/// ```
#[derive(Debug, Clone)]
pub struct RangeFilter {
    /// Start of ID range (inclusive)
    start_id: u32,
    /// End of ID range (inclusive)
    end_id: u32,
    /// Whether this is for extended IDs
    extended: bool,
}

impl RangeFilter {
    /// Create a new range filter for standard frames
    ///
    /// # Arguments
    ///
    /// * `start_id` - Start of the ID range (inclusive)
    /// * `end_id` - End of the ID range (inclusive)
    ///
    /// # Panics
    ///
    /// Panics if `start_id > end_id` or if IDs exceed maximum.
    #[must_use]
    pub fn new(start_id: u32, end_id: u32) -> Self {
        assert!(start_id <= end_id, "start_id must be <= end_id");
        assert!(end_id <= MAX_STANDARD_ID, "ID exceeds maximum standard ID");
        Self {
            start_id,
            end_id,
            extended: false,
        }
    }

    /// Create a new range filter for extended frames
    ///
    /// # Arguments
    ///
    /// * `start_id` - Start of the ID range (inclusive)
    /// * `end_id` - End of the ID range (inclusive)
    ///
    /// # Panics
    ///
    /// Panics if `start_id > end_id` or if IDs exceed maximum.
    #[must_use]
    pub fn new_extended(start_id: u32, end_id: u32) -> Self {
        assert!(start_id <= end_id, "start_id must be <= end_id");
        assert!(end_id <= MAX_EXTENDED_ID, "ID exceeds maximum extended ID");
        Self {
            start_id,
            end_id,
            extended: true,
        }
    }

    /// Try to create a new range filter, returning error if invalid
    ///
    /// # Errors
    ///
    /// Returns `FilterError::InvalidRange` if `start_id > end_id`.
    /// Returns `FilterError::IdOutOfRange` if `end_id` exceeds the maximum standard ID.
    pub fn try_new(start_id: u32, end_id: u32) -> Result<Self, FilterError> {
        if start_id > end_id {
            return Err(FilterError::InvalidRange {
                start: start_id,
                end: end_id,
            });
        }
        if end_id > MAX_STANDARD_ID {
            return Err(FilterError::IdOutOfRange {
                id: end_id,
                max: MAX_STANDARD_ID,
            });
        }
        Ok(Self::new(start_id, end_id))
    }

    /// Try to create a new extended range filter, returning error if invalid
    ///
    /// # Errors
    ///
    /// Returns `FilterError::InvalidRange` if `start_id > end_id`.
    /// Returns `FilterError::IdOutOfRange` if `end_id` exceeds the maximum extended ID.
    pub fn try_new_extended(start_id: u32, end_id: u32) -> Result<Self, FilterError> {
        if start_id > end_id {
            return Err(FilterError::InvalidRange {
                start: start_id,
                end: end_id,
            });
        }
        if end_id > MAX_EXTENDED_ID {
            return Err(FilterError::IdOutOfRange {
                id: end_id,
                max: MAX_EXTENDED_ID,
            });
        }
        Ok(Self::new_extended(start_id, end_id))
    }

    /// Get the start ID
    #[must_use]
    pub fn start_id(&self) -> u32 {
        self.start_id
    }

    /// Get the end ID
    #[must_use]
    pub fn end_id(&self) -> u32 {
        self.end_id
    }

    /// Check if this filter is for extended frames
    #[must_use]
    pub fn is_extended(&self) -> bool {
        self.extended
    }

    /// Get the range size
    #[must_use]
    pub fn range_size(&self) -> u32 {
        self.end_id - self.start_id + 1
    }
}

impl MessageFilter for RangeFilter {
    fn matches(&self, message: &CanMessage) -> bool {
        let msg_id = message.id();
        let is_extended = msg_id.is_extended();

        // Frame type must match
        if is_extended != self.extended {
            return false;
        }

        // Check if ID is in range
        let raw_id = msg_id.raw();
        raw_id >= self.start_id && raw_id <= self.end_id
    }

    fn priority(&self) -> u32 {
        // Smaller ranges have higher priority
        let range_size = self.end_id - self.start_id;
        if range_size == 0 {
            100 // Single ID, highest priority
        } else if range_size < 16 {
            75
        } else if range_size < 256 {
            50
        } else {
            25
        }
    }

    fn is_hardware(&self) -> bool {
        // Range filters are typically software-only
        // unless the range can be expressed as a mask
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::CanMessage;

    #[test]
    fn test_range_match() {
        let filter = RangeFilter::new(0x100, 0x1FF);

        let msg_100 = CanMessage::new_standard(0x100, &[0u8; 8]).unwrap();
        let msg_150 = CanMessage::new_standard(0x150, &[0u8; 8]).unwrap();
        let msg_1ff = CanMessage::new_standard(0x1FF, &[0u8; 8]).unwrap();
        let msg_200 = CanMessage::new_standard(0x200, &[0u8; 8]).unwrap();
        let msg_0ff = CanMessage::new_standard(0x0FF, &[0u8; 8]).unwrap();

        assert!(filter.matches(&msg_100));
        assert!(filter.matches(&msg_150));
        assert!(filter.matches(&msg_1ff));
        assert!(!filter.matches(&msg_200));
        assert!(!filter.matches(&msg_0ff));
    }

    #[test]
    fn test_single_id_range() {
        let filter = RangeFilter::new(0x123, 0x123);

        let msg_match = CanMessage::new_standard(0x123, &[0u8; 8]).unwrap();
        let msg_no_match = CanMessage::new_standard(0x124, &[0u8; 8]).unwrap();

        assert!(filter.matches(&msg_match));
        assert!(!filter.matches(&msg_no_match));
    }

    #[test]
    fn test_extended_range() {
        let filter = RangeFilter::new_extended(0x10000, 0x1FFFF);

        let msg_ext = CanMessage::new_extended(0x15000, &[0u8; 8]).unwrap();
        let msg_std = CanMessage::new_standard(0x100, &[0u8; 8]).unwrap();

        assert!(filter.matches(&msg_ext));
        assert!(!filter.matches(&msg_std)); // Frame type mismatch
    }

    #[test]
    fn test_try_new_invalid_range() {
        let result = RangeFilter::try_new(0x200, 0x100);
        assert!(result.is_err());
    }

    #[test]
    fn test_range_size() {
        let filter = RangeFilter::new(0x100, 0x1FF);
        assert_eq!(filter.range_size(), 256);
    }

    #[test]
    fn test_priority() {
        let single = RangeFilter::new(0x100, 0x100);
        let small = RangeFilter::new(0x100, 0x10F);
        let large = RangeFilter::new(0x100, 0x1FF);

        assert!(single.priority() > small.priority());
        assert!(small.priority() > large.priority());
    }
}
