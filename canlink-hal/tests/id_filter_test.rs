//! IdFilter unit tests (T027)
//!
//! Tests for single ID and mask-based filtering.

use canlink_hal::filter::{IdFilter, MessageFilter};
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
// Exact Match Tests
// ============================================================================

#[test]
fn test_exact_match_passes() {
    let filter = IdFilter::new(0x123);
    let msg = make_standard_message(0x123);
    assert!(filter.matches(&msg));
}

#[test]
fn test_exact_match_fails_different_id() {
    let filter = IdFilter::new(0x123);
    let msg = make_standard_message(0x456);
    assert!(!filter.matches(&msg));
}

#[test]
fn test_exact_match_zero_id() {
    let filter = IdFilter::new(0x000);
    let msg = make_standard_message(0x000);
    assert!(filter.matches(&msg));
}

#[test]
fn test_exact_match_max_standard_id() {
    let filter = IdFilter::new(0x7FF);
    let msg = make_standard_message(0x7FF);
    assert!(filter.matches(&msg));
}

// ============================================================================
// Mask Match Tests
// ============================================================================

#[test]
fn test_mask_match_lower_nibble_ignored() {
    // Mask 0x7F0 ignores lower 4 bits
    let filter = IdFilter::with_mask(0x120, 0x7F0);

    // Should match 0x120-0x12F
    assert!(filter.matches(&make_standard_message(0x120)));
    assert!(filter.matches(&make_standard_message(0x125)));
    assert!(filter.matches(&make_standard_message(0x12F)));

    // Should not match outside range
    assert!(!filter.matches(&make_standard_message(0x110)));
    assert!(!filter.matches(&make_standard_message(0x130)));
}

#[test]
fn test_mask_match_upper_bits_only() {
    // Mask 0x700 only checks upper 3 bits
    let filter = IdFilter::with_mask(0x100, 0x700);

    // Should match 0x100-0x1FF
    assert!(filter.matches(&make_standard_message(0x100)));
    assert!(filter.matches(&make_standard_message(0x1FF)));

    // Should not match 0x200+
    assert!(!filter.matches(&make_standard_message(0x200)));
}

#[test]
fn test_mask_match_full_mask_is_exact() {
    // Full mask (0x7FF) is equivalent to exact match
    let filter = IdFilter::with_mask(0x123, 0x7FF);
    assert!(filter.matches(&make_standard_message(0x123)));
    assert!(!filter.matches(&make_standard_message(0x124)));
}

#[test]
fn test_mask_match_zero_mask_matches_all() {
    // Zero mask matches all IDs
    let filter = IdFilter::with_mask(0x000, 0x000);
    assert!(filter.matches(&make_standard_message(0x000)));
    assert!(filter.matches(&make_standard_message(0x123)));
    assert!(filter.matches(&make_standard_message(0x7FF)));
}

// ============================================================================
// Standard vs Extended Frame Tests
// ============================================================================

#[test]
fn test_standard_filter_rejects_extended_frame() {
    let filter = IdFilter::new(0x123);
    let msg = make_extended_message(0x123);
    assert!(!filter.matches(&msg));
}

#[test]
fn test_extended_filter_rejects_standard_frame() {
    let filter = IdFilter::new_extended(0x123);
    let msg = make_standard_message(0x123);
    assert!(!filter.matches(&msg));
}

#[test]
fn test_extended_filter_matches_extended_frame() {
    let filter = IdFilter::new_extended(0x12345678);
    let msg = make_extended_message(0x12345678);
    assert!(filter.matches(&msg));
}

#[test]
fn test_extended_filter_max_id() {
    let filter = IdFilter::new_extended(0x1FFF_FFFF);
    let msg = make_extended_message(0x1FFF_FFFF);
    assert!(filter.matches(&msg));
}

#[test]
fn test_extended_mask_filter() {
    // Mask for extended frames
    let filter = IdFilter::with_mask_extended(0x12340000, 0x1FFF0000);

    assert!(filter.matches(&make_extended_message(0x12340000)));
    assert!(filter.matches(&make_extended_message(0x1234FFFF)));
    assert!(!filter.matches(&make_extended_message(0x12350000)));
}

// ============================================================================
// Boundary Value Tests
// ============================================================================

#[test]
fn test_boundary_standard_id_zero() {
    let filter = IdFilter::new(0);
    assert!(filter.matches(&make_standard_message(0)));
    assert!(!filter.matches(&make_standard_message(1)));
}

#[test]
fn test_boundary_standard_id_max() {
    let filter = IdFilter::new(0x7FF);
    assert!(filter.matches(&make_standard_message(0x7FF)));
    assert!(!filter.matches(&make_standard_message(0x7FE)));
}

#[test]
fn test_boundary_extended_id_zero() {
    let filter = IdFilter::new_extended(0);
    assert!(filter.matches(&make_extended_message(0)));
}

#[test]
fn test_boundary_extended_id_max() {
    let filter = IdFilter::new_extended(0x1FFF_FFFF);
    assert!(filter.matches(&make_extended_message(0x1FFF_FFFF)));
}

// ============================================================================
// Try Constructors Tests
// ============================================================================

#[test]
fn test_try_new_valid_id() {
    let filter = IdFilter::try_new(0x123);
    assert!(filter.is_ok());
}

#[test]
fn test_try_new_invalid_id() {
    let filter = IdFilter::try_new(0x800); // Exceeds max standard ID
    assert!(filter.is_err());
}

#[test]
fn test_try_new_extended_valid_id() {
    let filter = IdFilter::try_new_extended(0x1FFF_FFFF);
    assert!(filter.is_ok());
}

#[test]
fn test_try_new_extended_invalid_id() {
    let filter = IdFilter::try_new_extended(0x2000_0000); // Exceeds max extended ID
    assert!(filter.is_err());
}

// ============================================================================
// Priority Tests
// ============================================================================

#[test]
fn test_exact_match_has_higher_priority() {
    let exact = IdFilter::new(0x123);
    let masked = IdFilter::with_mask(0x120, 0x7F0);

    assert!(exact.priority() > masked.priority());
}

// ============================================================================
// Hardware Filter Tests
// ============================================================================

#[test]
fn test_id_filter_is_hardware_by_default() {
    let filter = IdFilter::new(0x123);
    assert!(filter.is_hardware());
}

#[test]
fn test_id_filter_hardware_can_be_disabled() {
    let mut filter = IdFilter::new(0x123);
    filter.set_hardware(false);
    assert!(!filter.is_hardware());
}

// ============================================================================
// Accessor Tests
// ============================================================================

#[test]
fn test_id_accessor() {
    let filter = IdFilter::new(0x123);
    assert_eq!(filter.id(), 0x123);
}

#[test]
fn test_mask_accessor() {
    let filter = IdFilter::with_mask(0x120, 0x7F0);
    assert_eq!(filter.mask(), 0x7F0);
}

#[test]
fn test_is_extended_accessor() {
    let standard = IdFilter::new(0x123);
    let extended = IdFilter::new_extended(0x123);

    assert!(!standard.is_extended());
    assert!(extended.is_extended());
}
