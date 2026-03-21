//! FilterChain unit tests (T029)
//!
//! Tests for filter chain management and evaluation.

use canlink_hal::filter::{FilterChain, IdFilter, RangeFilter};
use canlink_hal::message::CanMessage;

// ============================================================================
// Helper Functions
// ============================================================================

fn make_standard_message(id: u16) -> CanMessage {
    CanMessage::new_standard(id, &[0u8; 8]).unwrap()
}

// ============================================================================
// Empty Chain Tests
// ============================================================================

#[test]
fn test_empty_chain_passes_all() {
    let chain = FilterChain::new(4);

    assert!(chain.matches(&make_standard_message(0x000)));
    assert!(chain.matches(&make_standard_message(0x123)));
    assert!(chain.matches(&make_standard_message(0x7FF)));
}

#[test]
fn test_empty_chain_is_empty() {
    let chain = FilterChain::new(4);
    assert!(chain.is_empty());
    assert_eq!(chain.len(), 0);
}

#[test]
fn test_default_chain_is_empty() {
    let chain = FilterChain::default();
    assert!(chain.is_empty());
}

// ============================================================================
// Single Filter Tests
// ============================================================================

#[test]
fn test_single_id_filter_match() {
    let mut chain = FilterChain::new(4);
    chain.add_filter(Box::new(IdFilter::new(0x123)));

    assert!(chain.matches(&make_standard_message(0x123)));
    assert!(!chain.matches(&make_standard_message(0x456)));
}

#[test]
fn test_single_range_filter_match() {
    let mut chain = FilterChain::new(4);
    chain.add_filter(Box::new(RangeFilter::new(0x100, 0x1FF)));

    assert!(chain.matches(&make_standard_message(0x100)));
    assert!(chain.matches(&make_standard_message(0x150)));
    assert!(chain.matches(&make_standard_message(0x1FF)));
    assert!(!chain.matches(&make_standard_message(0x200)));
}

// ============================================================================
// Multiple Filters OR Logic Tests
// ============================================================================

#[test]
fn test_multiple_id_filters_or_logic() {
    let mut chain = FilterChain::new(4);
    chain.add_filter(Box::new(IdFilter::new(0x100)));
    chain.add_filter(Box::new(IdFilter::new(0x200)));
    chain.add_filter(Box::new(IdFilter::new(0x300)));

    // Should match any of the IDs
    assert!(chain.matches(&make_standard_message(0x100)));
    assert!(chain.matches(&make_standard_message(0x200)));
    assert!(chain.matches(&make_standard_message(0x300)));

    // Should not match other IDs
    assert!(!chain.matches(&make_standard_message(0x150)));
    assert!(!chain.matches(&make_standard_message(0x400)));
}

#[test]
fn test_mixed_filters_or_logic() {
    let mut chain = FilterChain::new(4);
    chain.add_filter(Box::new(IdFilter::new(0x123)));
    chain.add_filter(Box::new(RangeFilter::new(0x200, 0x2FF)));

    // Should match exact ID
    assert!(chain.matches(&make_standard_message(0x123)));

    // Should match range
    assert!(chain.matches(&make_standard_message(0x200)));
    assert!(chain.matches(&make_standard_message(0x250)));
    assert!(chain.matches(&make_standard_message(0x2FF)));

    // Should not match outside both
    assert!(!chain.matches(&make_standard_message(0x100)));
    assert!(!chain.matches(&make_standard_message(0x300)));
}

#[test]
fn test_overlapping_filters() {
    let mut chain = FilterChain::new(4);
    chain.add_filter(Box::new(RangeFilter::new(0x100, 0x1FF)));
    chain.add_filter(Box::new(RangeFilter::new(0x150, 0x250)));

    // Should match in first range only
    assert!(chain.matches(&make_standard_message(0x100)));

    // Should match in overlap
    assert!(chain.matches(&make_standard_message(0x180)));

    // Should match in second range only
    assert!(chain.matches(&make_standard_message(0x220)));

    // Should not match outside both
    assert!(!chain.matches(&make_standard_message(0x050)));
    assert!(!chain.matches(&make_standard_message(0x300)));
}

// ============================================================================
// Hardware Filter Fallback Tests
// ============================================================================

#[test]
fn test_hardware_filter_count_tracking() {
    let mut chain = FilterChain::new(2); // Max 2 hardware filters

    // IdFilter is hardware by default
    chain.add_filter(Box::new(IdFilter::new(0x100)));
    assert_eq!(chain.hardware_filter_count(), 1);

    chain.add_filter(Box::new(IdFilter::new(0x200)));
    assert_eq!(chain.hardware_filter_count(), 2);

    // Third hardware filter should not increase count (at capacity)
    chain.add_filter(Box::new(IdFilter::new(0x300)));
    assert_eq!(chain.hardware_filter_count(), 2);
    assert_eq!(chain.len(), 3);
}

#[test]
fn test_software_filter_not_counted_as_hardware() {
    let mut chain = FilterChain::new(4);

    // RangeFilter is software by default
    chain.add_filter(Box::new(RangeFilter::new(0x100, 0x1FF)));
    assert_eq!(chain.hardware_filter_count(), 0);
    assert_eq!(chain.software_filter_count(), 1);
}

#[test]
fn test_has_hardware_capacity() {
    let mut chain = FilterChain::new(2);

    assert!(chain.has_hardware_capacity());

    chain.add_filter(Box::new(IdFilter::new(0x100)));
    assert!(chain.has_hardware_capacity());

    chain.add_filter(Box::new(IdFilter::new(0x200)));
    assert!(!chain.has_hardware_capacity());
}

#[test]
fn test_max_hardware_filters_accessor() {
    let chain = FilterChain::new(8);
    assert_eq!(chain.max_hardware_filters(), 8);
}

// ============================================================================
// Remove Filter Tests
// ============================================================================

#[test]
fn test_remove_filter_by_index() {
    let mut chain = FilterChain::new(4);
    chain.add_filter(Box::new(IdFilter::new(0x100)));
    chain.add_filter(Box::new(IdFilter::new(0x200)));
    chain.add_filter(Box::new(IdFilter::new(0x300)));

    assert_eq!(chain.len(), 3);

    // Remove middle filter
    let removed = chain.remove_filter(1);
    assert!(removed.is_ok());
    assert_eq!(chain.len(), 2);

    // 0x200 should no longer match
    assert!(chain.matches(&make_standard_message(0x100)));
    assert!(!chain.matches(&make_standard_message(0x200)));
    assert!(chain.matches(&make_standard_message(0x300)));
}

#[test]
fn test_remove_filter_updates_hardware_count() {
    let mut chain = FilterChain::new(4);
    chain.add_filter(Box::new(IdFilter::new(0x100)));
    chain.add_filter(Box::new(IdFilter::new(0x200)));

    assert_eq!(chain.hardware_filter_count(), 2);

    chain.remove_filter(0).unwrap();
    assert_eq!(chain.hardware_filter_count(), 1);
}

#[test]
fn test_remove_filter_invalid_index() {
    let mut chain = FilterChain::new(4);
    chain.add_filter(Box::new(IdFilter::new(0x100)));

    let result = chain.remove_filter(5);
    assert!(result.is_err());
}

#[test]
fn test_remove_from_empty_chain() {
    let mut chain = FilterChain::new(4);
    let result = chain.remove_filter(0);
    assert!(result.is_err());
}

// ============================================================================
// Clear Tests
// ============================================================================

#[test]
fn test_clear_removes_all_filters() {
    let mut chain = FilterChain::new(4);
    chain.add_filter(Box::new(IdFilter::new(0x100)));
    chain.add_filter(Box::new(IdFilter::new(0x200)));
    chain.add_filter(Box::new(RangeFilter::new(0x300, 0x3FF)));

    assert_eq!(chain.len(), 3);

    chain.clear();

    assert!(chain.is_empty());
    assert_eq!(chain.len(), 0);
    assert_eq!(chain.hardware_filter_count(), 0);
}

#[test]
fn test_clear_then_passes_all() {
    let mut chain = FilterChain::new(4);
    chain.add_filter(Box::new(IdFilter::new(0x100)));

    // Before clear, only 0x100 matches
    assert!(chain.matches(&make_standard_message(0x100)));
    assert!(!chain.matches(&make_standard_message(0x200)));

    chain.clear();

    // After clear, all messages pass
    assert!(chain.matches(&make_standard_message(0x100)));
    assert!(chain.matches(&make_standard_message(0x200)));
}

// ============================================================================
// Iterator Tests
// ============================================================================

#[test]
fn test_iter_over_filters() {
    let mut chain = FilterChain::new(4);
    chain.add_filter(Box::new(IdFilter::new(0x100)));
    chain.add_filter(Box::new(IdFilter::new(0x200)));

    let count = chain.iter().count();
    assert_eq!(count, 2);
}

// ============================================================================
// Total Filter Count Tests
// ============================================================================

#[test]
fn test_total_filter_count() {
    let mut chain = FilterChain::new(4);

    assert_eq!(chain.total_filter_count(), 0);

    chain.add_filter(Box::new(IdFilter::new(0x100)));
    assert_eq!(chain.total_filter_count(), 1);

    chain.add_filter(Box::new(RangeFilter::new(0x200, 0x2FF)));
    assert_eq!(chain.total_filter_count(), 2);
}

// ============================================================================
// Complex Scenarios
// ============================================================================

#[test]
fn test_many_filters() {
    let mut chain = FilterChain::new(16);

    // Add many ID filters (staying within standard ID range 0x000-0x7FF)
    for i in 0..8u32 {
        chain.add_filter(Box::new(IdFilter::new(i * 0x100)));
    }

    assert_eq!(chain.len(), 8);

    // Should match all added IDs
    for i in 0..8u16 {
        assert!(chain.matches(&make_standard_message(i * 0x100)));
    }

    // Should not match IDs in between
    assert!(!chain.matches(&make_standard_message(0x050)));
    assert!(!chain.matches(&make_standard_message(0x150)));
}

#[test]
fn test_filter_chain_with_mask_filter() {
    let mut chain = FilterChain::new(4);

    // Add mask filter that matches 0x120-0x12F
    chain.add_filter(Box::new(IdFilter::with_mask(0x120, 0x7F0)));

    assert!(chain.matches(&make_standard_message(0x120)));
    assert!(chain.matches(&make_standard_message(0x12F)));
    assert!(!chain.matches(&make_standard_message(0x130)));
}

#[test]
fn test_rebuild_chain_after_clear() {
    let mut chain = FilterChain::new(4);

    // First configuration
    chain.add_filter(Box::new(IdFilter::new(0x100)));
    assert!(chain.matches(&make_standard_message(0x100)));
    assert!(!chain.matches(&make_standard_message(0x200)));

    // Clear and reconfigure
    chain.clear();
    chain.add_filter(Box::new(IdFilter::new(0x200)));

    // New configuration
    assert!(!chain.matches(&make_standard_message(0x100)));
    assert!(chain.matches(&make_standard_message(0x200)));
}
