//! Filter chain implementation (FR-006)
//!
//! Manages multiple filters with automatic hardware/software fallback.

use crate::error::FilterError;
use crate::message::CanMessage;

use super::MessageFilter;

/// Filter chain for managing multiple filters
///
/// The filter chain manages a collection of filters and automatically
/// handles hardware filter limits by falling back to software filtering.
///
/// # Filter Evaluation
///
/// Filters are evaluated in order of priority (highest first).
/// A message passes if ANY filter matches (OR logic).
/// An empty chain passes all messages.
///
/// # Hardware Filter Management
///
/// The chain tracks hardware filter capacity. When the limit is reached,
/// additional filters are automatically treated as software filters.
///
/// # Example
///
/// ```rust
/// use canlink_hal::filter::{FilterChain, IdFilter, RangeFilter};
///
/// let mut chain = FilterChain::new(4); // Max 4 hardware filters
///
/// chain.add_filter(Box::new(IdFilter::new(0x123)));
/// chain.add_filter(Box::new(RangeFilter::new(0x200, 0x2FF)));
///
/// // Check if a message passes the filter chain
/// // let passes = chain.matches(&message);
/// ```
pub struct FilterChain {
    /// All filters in the chain
    filters: Vec<Box<dyn MessageFilter>>,
    /// Maximum number of hardware filters supported
    max_hardware_filters: usize,
    /// Current number of hardware filters in use
    hardware_filter_count: usize,
}

impl FilterChain {
    /// Create a new filter chain
    ///
    /// # Arguments
    ///
    /// * `max_hardware_filters` - Maximum number of hardware filters supported
    #[must_use]
    pub fn new(max_hardware_filters: usize) -> Self {
        Self {
            filters: Vec::new(),
            max_hardware_filters,
            hardware_filter_count: 0,
        }
    }

    /// Add a filter to the chain
    ///
    /// Filters are added in order. If the filter is a hardware filter
    /// and the hardware limit has been reached, it will be treated as
    /// a software filter.
    ///
    /// # Arguments
    ///
    /// * `filter` - The filter to add
    pub fn add_filter(&mut self, filter: Box<dyn MessageFilter>) {
        if filter.is_hardware() && self.hardware_filter_count < self.max_hardware_filters {
            self.hardware_filter_count += 1;
        }
        self.filters.push(filter);
    }

    /// Remove a filter at the given index
    ///
    /// # Arguments
    ///
    /// * `index` - Index of the filter to remove
    ///
    /// # Errors
    ///
    /// Returns `FilterError::FilterNotFound` if the index is out of bounds.
    pub fn remove_filter(&mut self, index: usize) -> Result<Box<dyn MessageFilter>, FilterError> {
        if index >= self.filters.len() {
            return Err(FilterError::FilterNotFound { index });
        }

        let filter = self.filters.remove(index);
        if filter.is_hardware() && self.hardware_filter_count > 0 {
            self.hardware_filter_count -= 1;
        }
        Ok(filter)
    }

    /// Clear all filters from the chain
    pub fn clear(&mut self) {
        self.filters.clear();
        self.hardware_filter_count = 0;
    }

    /// Check if a message matches any filter in the chain
    ///
    /// Returns `true` if:
    /// - The chain is empty (pass-through mode)
    /// - Any filter in the chain matches the message
    #[must_use]
    pub fn matches(&self, message: &CanMessage) -> bool {
        // Empty chain passes all messages
        if self.filters.is_empty() {
            return true;
        }

        // Check if any filter matches (OR logic)
        self.filters.iter().any(|f| f.matches(message))
    }

    /// Get the total number of filters
    #[must_use]
    pub fn len(&self) -> usize {
        self.filters.len()
    }

    /// Check if the chain is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.filters.is_empty()
    }

    /// Get the number of hardware filters in use
    #[must_use]
    pub fn hardware_filter_count(&self) -> usize {
        self.hardware_filter_count
    }

    /// Get the number of software filters in use
    #[must_use]
    pub fn software_filter_count(&self) -> usize {
        self.filters
            .iter()
            .filter(|f| !f.is_hardware() || self.hardware_filter_count >= self.max_hardware_filters)
            .count()
    }

    /// Get the maximum hardware filter capacity
    #[must_use]
    pub fn max_hardware_filters(&self) -> usize {
        self.max_hardware_filters
    }

    /// Check if hardware filter capacity is available
    #[must_use]
    pub fn has_hardware_capacity(&self) -> bool {
        self.hardware_filter_count < self.max_hardware_filters
    }

    /// Get the total filter count (alias for len)
    #[must_use]
    pub fn total_filter_count(&self) -> usize {
        self.len()
    }

    /// Iterate over filters
    pub fn iter(&self) -> impl Iterator<Item = &dyn MessageFilter> {
        self.filters.iter().map(std::convert::AsRef::as_ref)
    }
}

impl Default for FilterChain {
    fn default() -> Self {
        Self::new(4) // Default to 4 hardware filters
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::{IdFilter, RangeFilter};
    use crate::message::CanMessage;

    fn make_message(id: u16) -> CanMessage {
        CanMessage::new_standard(id, &[0u8; 8]).unwrap()
    }

    #[test]
    fn test_empty_chain_passes_all() {
        let chain = FilterChain::new(4);
        assert!(chain.matches(&make_message(0x123)));
        assert!(chain.matches(&make_message(0x456)));
    }

    #[test]
    fn test_single_filter() {
        let mut chain = FilterChain::new(4);
        chain.add_filter(Box::new(IdFilter::new(0x123)));

        assert!(chain.matches(&make_message(0x123)));
        assert!(!chain.matches(&make_message(0x456)));
    }

    #[test]
    fn test_multiple_filters_or_logic() {
        let mut chain = FilterChain::new(4);
        chain.add_filter(Box::new(IdFilter::new(0x123)));
        chain.add_filter(Box::new(IdFilter::new(0x456)));

        assert!(chain.matches(&make_message(0x123)));
        assert!(chain.matches(&make_message(0x456)));
        assert!(!chain.matches(&make_message(0x789)));
    }

    #[test]
    fn test_mixed_filters() {
        let mut chain = FilterChain::new(4);
        chain.add_filter(Box::new(IdFilter::new(0x123)));
        chain.add_filter(Box::new(RangeFilter::new(0x200, 0x2FF)));

        assert!(chain.matches(&make_message(0x123)));
        assert!(chain.matches(&make_message(0x250)));
        assert!(!chain.matches(&make_message(0x300)));
    }

    #[test]
    fn test_hardware_filter_count() {
        let mut chain = FilterChain::new(2);

        // IdFilter is hardware by default
        chain.add_filter(Box::new(IdFilter::new(0x100)));
        assert_eq!(chain.hardware_filter_count(), 1);

        chain.add_filter(Box::new(IdFilter::new(0x200)));
        assert_eq!(chain.hardware_filter_count(), 2);

        // RangeFilter is software by default
        chain.add_filter(Box::new(RangeFilter::new(0x300, 0x3FF)));
        assert_eq!(chain.hardware_filter_count(), 2);
    }

    #[test]
    fn test_remove_filter() {
        let mut chain = FilterChain::new(4);
        chain.add_filter(Box::new(IdFilter::new(0x123)));
        chain.add_filter(Box::new(IdFilter::new(0x456)));

        assert_eq!(chain.len(), 2);

        chain.remove_filter(0).unwrap();
        assert_eq!(chain.len(), 1);

        // Now only 0x456 should match
        assert!(!chain.matches(&make_message(0x123)));
        assert!(chain.matches(&make_message(0x456)));
    }

    #[test]
    fn test_remove_invalid_index() {
        let mut chain = FilterChain::new(4);
        let result = chain.remove_filter(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_clear() {
        let mut chain = FilterChain::new(4);
        chain.add_filter(Box::new(IdFilter::new(0x123)));
        chain.add_filter(Box::new(IdFilter::new(0x456)));

        chain.clear();
        assert!(chain.is_empty());
        assert_eq!(chain.hardware_filter_count(), 0);
    }
}
