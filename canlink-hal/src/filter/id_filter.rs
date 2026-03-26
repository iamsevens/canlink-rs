//! ID filter implementation (FR-006)
//!
//! Provides single ID and mask-based filtering.

use crate::error::FilterError;
use crate::message::CanMessage;

use super::MessageFilter;

/// Maximum standard CAN ID (11-bit)
pub const MAX_STANDARD_ID: u32 = 0x7FF;

/// Maximum extended CAN ID (29-bit)
pub const MAX_EXTENDED_ID: u32 = 0x1FFF_FFFF;

/// ID filter for single ID or mask-based matching
///
/// Supports both exact ID matching and mask-based matching.
///
/// # Example
///
/// ```rust
/// use canlink_hal::filter::IdFilter;
///
/// // Exact match for ID 0x123
/// let filter = IdFilter::new(0x123);
///
/// // Mask match: matches 0x120-0x12F
/// let filter = IdFilter::with_mask(0x120, 0x7F0);
/// ```
#[derive(Debug, Clone)]
pub struct IdFilter {
    /// Target ID
    id: u32,
    /// Mask for matching (0xFFFFFFFF for exact match)
    mask: u32,
    /// Whether this is for extended IDs
    extended: bool,
    /// Whether this can be a hardware filter
    hardware: bool,
}

impl IdFilter {
    /// Create a new filter for exact ID match (standard frame)
    ///
    /// # Arguments
    ///
    /// * `id` - The CAN ID to match (must be <= 0x7FF)
    ///
    /// # Panics
    ///
    /// Panics if `id` exceeds the maximum standard ID.
    #[must_use]
    pub fn new(id: u32) -> Self {
        assert!(id <= MAX_STANDARD_ID, "ID exceeds maximum standard ID");
        Self {
            id,
            mask: MAX_STANDARD_ID,
            extended: false,
            hardware: true,
        }
    }

    /// Create a new filter with mask (standard frame)
    ///
    /// # Arguments
    ///
    /// * `id` - The target ID
    /// * `mask` - Bits set to 1 must match, bits set to 0 are ignored
    ///
    /// # Example
    ///
    /// ```rust
    /// use canlink_hal::filter::IdFilter;
    ///
    /// // Match IDs 0x120-0x12F (mask ignores lower 4 bits)
    /// let filter = IdFilter::with_mask(0x120, 0x7F0);
    /// ```
    #[must_use]
    pub fn with_mask(id: u32, mask: u32) -> Self {
        Self {
            id: id & mask,
            mask,
            extended: false,
            hardware: true,
        }
    }

    /// Create a new filter for extended frame
    ///
    /// # Arguments
    ///
    /// * `id` - The CAN ID to match (must be <= 0x1FFFFFFF)
    ///
    /// # Panics
    ///
    /// Panics if `id` exceeds the maximum extended ID.
    #[must_use]
    pub fn new_extended(id: u32) -> Self {
        assert!(id <= MAX_EXTENDED_ID, "ID exceeds maximum extended ID");
        Self {
            id,
            mask: MAX_EXTENDED_ID,
            extended: true,
            hardware: true,
        }
    }

    /// Create a new filter with mask for extended frame
    #[must_use]
    pub fn with_mask_extended(id: u32, mask: u32) -> Self {
        Self {
            id: id & mask,
            mask,
            extended: true,
            hardware: true,
        }
    }

    /// Try to create a new filter, returning error if ID is invalid
    ///
    /// # Errors
    ///
    /// Returns `FilterError::IdOutOfRange` if `id` exceeds the maximum standard ID.
    pub fn try_new(id: u32) -> Result<Self, FilterError> {
        if id > MAX_STANDARD_ID {
            return Err(FilterError::IdOutOfRange {
                id,
                max: MAX_STANDARD_ID,
            });
        }
        Ok(Self::new(id))
    }

    /// Try to create a new extended filter, returning error if ID is invalid
    ///
    /// # Errors
    ///
    /// Returns `FilterError::IdOutOfRange` if `id` exceeds the maximum extended ID.
    pub fn try_new_extended(id: u32) -> Result<Self, FilterError> {
        if id > MAX_EXTENDED_ID {
            return Err(FilterError::IdOutOfRange {
                id,
                max: MAX_EXTENDED_ID,
            });
        }
        Ok(Self::new_extended(id))
    }

    /// Set whether this filter can use hardware acceleration
    pub fn set_hardware(&mut self, hardware: bool) {
        self.hardware = hardware;
    }

    /// Get the target ID
    #[must_use]
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get the mask
    #[must_use]
    pub fn mask(&self) -> u32 {
        self.mask
    }

    /// Check if this filter is for extended frames
    #[must_use]
    pub fn is_extended(&self) -> bool {
        self.extended
    }
}

impl MessageFilter for IdFilter {
    fn matches(&self, message: &CanMessage) -> bool {
        let msg_id = message.id();
        let is_extended = msg_id.is_extended();

        // Frame type must match
        if is_extended != self.extended {
            return false;
        }

        // Apply mask and compare
        let raw_id = msg_id.raw();
        (raw_id & self.mask) == (self.id & self.mask)
    }

    fn priority(&self) -> u32 {
        // Exact match (full mask) has higher priority
        if self.mask == MAX_STANDARD_ID || self.mask == MAX_EXTENDED_ID {
            100
        } else {
            50
        }
    }

    fn is_hardware(&self) -> bool {
        self.hardware
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::CanMessage;

    #[test]
    fn test_exact_match() {
        let filter = IdFilter::new(0x123);

        let msg_match = CanMessage::new_standard(0x123, &[0u8; 8]).unwrap();
        let msg_no_match = CanMessage::new_standard(0x456, &[0u8; 8]).unwrap();

        assert!(filter.matches(&msg_match));
        assert!(!filter.matches(&msg_no_match));
    }

    #[test]
    fn test_mask_match() {
        // Match 0x120-0x12F
        let filter = IdFilter::with_mask(0x120, 0x7F0);

        let msg_120 = CanMessage::new_standard(0x120, &[0u8; 8]).unwrap();
        let msg_12f_id = CanMessage::new_standard(0x12F, &[0u8; 8]).unwrap();
        let msg_130 = CanMessage::new_standard(0x130, &[0u8; 8]).unwrap();

        assert!(filter.matches(&msg_120));
        assert!(filter.matches(&msg_12f_id));
        assert!(!filter.matches(&msg_130));
    }

    #[test]
    fn test_extended_filter() {
        let filter = IdFilter::new_extended(0x1234_5678);

        let msg_ext = CanMessage::new_extended(0x1234_5678, &[0u8; 8]).unwrap();
        let msg_std = CanMessage::new_standard(0x123, &[0u8; 8]).unwrap();

        assert!(filter.matches(&msg_ext));
        assert!(!filter.matches(&msg_std)); // Frame type mismatch
    }

    #[test]
    fn test_try_new_invalid() {
        let result = IdFilter::try_new(0x800);
        assert!(result.is_err());
    }

    #[test]
    fn test_priority() {
        let exact = IdFilter::new(0x123);
        let masked = IdFilter::with_mask(0x120, 0x7F0);

        assert!(exact.priority() > masked.priority());
    }
}
