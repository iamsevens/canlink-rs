//! RangeFilter unit tests (T028)
//!
//! Tests for ID range-based filtering.

use canlink_hal::filter::{MessageFilter, RangeFilter};
use canlink_hal::message::CanMessage;

// ============================================================================
// Helper Functions
// ============================================================================

fn make_standard_message(id: u16) -> CanMessage {
    CanMessage::new_standard(id, &[0u8; 8]).unwrap()
}

fn make_extended_message(id: u32) -> CanMessage {
    CanMessage::new_extended(id, &[0u8; 8]).unwrap()
}

// ============================================================================
// Range Match Tests
// ============================================================================

#[test]
fn test_range_match_start() {
    let filter = RangeFilter::new(0x100, 0x1FF);
    assert!(filter.matches(&make_standard_message(0x100)));
}

#[test]
fn test_range_match_end() {
    let filter = RangeFilter::new(0x100, 0x1FF);
    assert!(filter.matches(&make_standard_message(0x1FF)));
}

#[test]
fn test_range_match_middle() {
    let filter = RangeFilter::new(0x100, 0x1FF);
    assert!(filter.matches(&make_standard_message(0x150)));
}

#[test]
fn test_range_no_match_below() {
    let filter = RangeFilter::new(0x100, 0x1FF);
    assert!(!filter.matches(&make_standard_message(0x0FF)));
}

#[test]
fn test_range_no_match_above() {
    let filter = RangeFilter::new(0x100, 0x1FF);
    assert!(!filter.matches(&make_standard_message(0x200)));
}

// ============================================================================
// Range Boundary Tests
// ============================================================================

#[test]
fn test_range_boundary_just_below_start() {
    let filter = RangeFilter::new(0x100, 0x1FF);
    assert!(!filter.matches(&make_standard_message(0x0FF)));
    assert!(filter.matches(&make_standard_message(0x100)));
}

#[test]
fn test_range_boundary_just_above_end() {
    let filter = RangeFilter::new(0x100, 0x1FF);
    assert!(filter.matches(&make_standard_message(0x1FF)));
    assert!(!filter.matches(&make_standard_message(0x200)));
}

#[test]
fn test_range_zero_start() {
    let filter = RangeFilter::new(0x000, 0x0FF);
    assert!(filter.matches(&make_standard_message(0x000)));
    assert!(filter.matches(&make_standard_message(0x0FF)));
    assert!(!filter.matches(&make_standard_message(0x100)));
}

#[test]
fn test_range_max_end() {
    let filter = RangeFilter::new(0x700, 0x7FF);
    assert!(filter.matches(&make_standard_message(0x700)));
    assert!(filter.matches(&make_standard_message(0x7FF)));
}

// ============================================================================
// Single ID Range Tests
// ============================================================================

#[test]
fn test_single_id_range() {
    let filter = RangeFilter::new(0x123, 0x123);
    assert!(filter.matches(&make_standard_message(0x123)));
    assert!(!filter.matches(&make_standard_message(0x122)));
    assert!(!filter.matches(&make_standard_message(0x124)));
}

#[test]
fn test_single_id_range_zero() {
    let filter = RangeFilter::new(0x000, 0x000);
    assert!(filter.matches(&make_standard_message(0x000)));
    assert!(!filter.matches(&make_standard_message(0x001)));
}

#[test]
fn test_single_id_range_max() {
    let filter = RangeFilter::new(0x7FF, 0x7FF);
    assert!(filter.matches(&make_standard_message(0x7FF)));
    assert!(!filter.matches(&make_standard_message(0x7FE)));
}

// ============================================================================
// Extended Frame Range Tests
// ============================================================================

#[test]
fn test_extended_range_match() {
    let filter = RangeFilter::new_extended(0x10000, 0x1FFFF);
    assert!(filter.matches(&make_extended_message(0x10000)));
    assert!(filter.matches(&make_extended_message(0x15000)));
    assert!(filter.matches(&make_extended_message(0x1FFFF)));
}

#[test]
fn test_extended_range_no_match_below() {
    let filter = RangeFilter::new_extended(0x10000, 0x1FFFF);
    assert!(!filter.matches(&make_extended_message(0x0FFFF)));
}

#[test]
fn test_extended_range_no_match_above() {
    let filter = RangeFilter::new_extended(0x10000, 0x1FFFF);
    assert!(!filter.matches(&make_extended_message(0x20000)));
}

#[test]
fn test_extended_range_rejects_standard_frame() {
    let filter = RangeFilter::new_extended(0x100, 0x1FF);
    // Standard frame with same ID should not match extended filter
    assert!(!filter.matches(&make_standard_message(0x150)));
}

#[test]
fn test_standard_range_rejects_extended_frame() {
    let filter = RangeFilter::new(0x100, 0x1FF);
    // Extended frame with same ID should not match standard filter
    assert!(!filter.matches(&make_extended_message(0x150)));
}

// ============================================================================
// Try Constructors Tests
// ============================================================================

#[test]
fn test_try_new_valid_range() {
    let filter = RangeFilter::try_new(0x100, 0x1FF);
    assert!(filter.is_ok());
}

#[test]
fn test_try_new_invalid_range_reversed() {
    let filter = RangeFilter::try_new(0x200, 0x100);
    assert!(filter.is_err());
}

#[test]
fn test_try_new_invalid_range_exceeds_max() {
    let filter = RangeFilter::try_new(0x100, 0x800);
    assert!(filter.is_err());
}

#[test]
fn test_try_new_extended_valid_range() {
    let filter = RangeFilter::try_new_extended(0x10000, 0x1FFFF);
    assert!(filter.is_ok());
}

#[test]
fn test_try_new_extended_invalid_range_reversed() {
    let filter = RangeFilter::try_new_extended(0x20000, 0x10000);
    assert!(filter.is_err());
}

#[test]
fn test_try_new_extended_invalid_range_exceeds_max() {
    let filter = RangeFilter::try_new_extended(0x10000, 0x2000_0000);
    assert!(filter.is_err());
}

// ============================================================================
// Range Size Tests
// ============================================================================

#[test]
fn test_range_size_single() {
    let filter = RangeFilter::new(0x123, 0x123);
    assert_eq!(filter.range_size(), 1);
}

#[test]
fn test_range_size_small() {
    let filter = RangeFilter::new(0x100, 0x10F);
    assert_eq!(filter.range_size(), 16);
}

#[test]
fn test_range_size_256() {
    let filter = RangeFilter::new(0x100, 0x1FF);
    assert_eq!(filter.range_size(), 256);
}

#[test]
fn test_range_size_full_standard() {
    let filter = RangeFilter::new(0x000, 0x7FF);
    assert_eq!(filter.range_size(), 2048);
}

// ============================================================================
// Priority Tests
// ============================================================================

#[test]
fn test_single_id_range_highest_priority() {
    let single = RangeFilter::new(0x100, 0x100);
    let small = RangeFilter::new(0x100, 0x10F);
    let large = RangeFilter::new(0x100, 0x1FF);

    assert!(single.priority() > small.priority());
    assert!(small.priority() > large.priority());
}

#[test]
fn test_priority_tiers() {
    // Single ID (range_size = 1) -> priority 100
    let single = RangeFilter::new(0x100, 0x100);
    assert_eq!(single.priority(), 100);

    // Small range (< 16) -> priority 75
    let small = RangeFilter::new(0x100, 0x10E); // size = 15
    assert_eq!(small.priority(), 75);

    // Medium range (< 256) -> priority 50
    let medium = RangeFilter::new(0x100, 0x1FF); // size = 256
    assert_eq!(medium.priority(), 50);

    // Large range (>= 256) -> priority 25
    let large = RangeFilter::new(0x000, 0x7FF); // size = 2048
    assert_eq!(large.priority(), 25);
}

// ============================================================================
// Hardware Filter Tests
// ============================================================================

#[test]
fn test_range_filter_is_software_by_default() {
    let filter = RangeFilter::new(0x100, 0x1FF);
    assert!(!filter.is_hardware());
}

// ============================================================================
// Accessor Tests
// ============================================================================

#[test]
fn test_start_id_accessor() {
    let filter = RangeFilter::new(0x100, 0x1FF);
    assert_eq!(filter.start_id(), 0x100);
}

#[test]
fn test_end_id_accessor() {
    let filter = RangeFilter::new(0x100, 0x1FF);
    assert_eq!(filter.end_id(), 0x1FF);
}

#[test]
fn test_is_extended_accessor() {
    let standard = RangeFilter::new(0x100, 0x1FF);
    let extended = RangeFilter::new_extended(0x10000, 0x1FFFF);

    assert!(!standard.is_extended());
    assert!(extended.is_extended());
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_full_standard_range() {
    let filter = RangeFilter::new(0x000, 0x7FF);

    // Should match all standard IDs
    assert!(filter.matches(&make_standard_message(0x000)));
    assert!(filter.matches(&make_standard_message(0x123)));
    assert!(filter.matches(&make_standard_message(0x7FF)));
}

#[test]
fn test_adjacent_ranges_no_overlap() {
    let filter1 = RangeFilter::new(0x100, 0x1FF);
    let filter2 = RangeFilter::new(0x200, 0x2FF);

    let msg_1ff = make_standard_message(0x1FF);
    let msg_200 = make_standard_message(0x200);

    assert!(filter1.matches(&msg_1ff));
    assert!(!filter1.matches(&msg_200));

    assert!(!filter2.matches(&msg_1ff));
    assert!(filter2.matches(&msg_200));
}
