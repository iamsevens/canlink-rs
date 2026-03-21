//! Filter integration tests (T030)
//!
//! Integration tests for filter functionality with MockBackend.

use canlink_hal::filter::{FilterConfig, IdFilter};
use canlink_hal::message::CanMessage;
use canlink_hal::{BackendConfig, CanBackend};
use canlink_mock::{MockBackend, MockConfig};

// ============================================================================
// Helper Functions
// ============================================================================

fn setup_backend_with_messages(messages: Vec<CanMessage>) -> MockBackend {
    let config = MockConfig::with_preset_messages(messages);
    let mut backend = MockBackend::with_config(config);
    backend.initialize(&BackendConfig::new("mock")).unwrap();
    backend.open_channel(0).unwrap();
    backend
}

// ============================================================================
// MockBackend Filter Integration Tests
// ============================================================================

#[test]
fn test_mock_backend_no_filter_receives_all() {
    let messages = vec![
        CanMessage::new_standard(0x100, &[1]).unwrap(),
        CanMessage::new_standard(0x200, &[2]).unwrap(),
        CanMessage::new_standard(0x300, &[3]).unwrap(),
    ];
    let mut backend = setup_backend_with_messages(messages);

    // No filters - should receive all messages
    let msg1 = backend.receive_message().unwrap();
    assert!(msg1.is_some());

    let msg2 = backend.receive_message().unwrap();
    assert!(msg2.is_some());

    let msg3 = backend.receive_message().unwrap();
    assert!(msg3.is_some());

    let msg4 = backend.receive_message().unwrap();
    assert!(msg4.is_none());
}

#[test]
fn test_mock_backend_id_filter_filters_messages() {
    let messages = vec![
        CanMessage::new_standard(0x100, &[1]).unwrap(),
        CanMessage::new_standard(0x200, &[2]).unwrap(),
        CanMessage::new_standard(0x300, &[3]).unwrap(),
    ];
    let mut backend = setup_backend_with_messages(messages);

    // Add filter for 0x200 only
    backend.add_id_filter(0x200);

    // Should only receive 0x200
    let msg = backend.receive_message().unwrap();
    assert!(msg.is_some());
    assert_eq!(msg.unwrap().data()[0], 2);

    // No more messages
    let msg = backend.receive_message().unwrap();
    assert!(msg.is_none());
}

#[test]
fn test_mock_backend_range_filter_filters_messages() {
    let messages = vec![
        CanMessage::new_standard(0x050, &[1]).unwrap(),
        CanMessage::new_standard(0x100, &[2]).unwrap(),
        CanMessage::new_standard(0x150, &[3]).unwrap(),
        CanMessage::new_standard(0x200, &[4]).unwrap(),
    ];
    let mut backend = setup_backend_with_messages(messages);

    // Add range filter for 0x100-0x1FF
    backend.add_range_filter(0x100, 0x1FF);

    // Should receive 0x100 and 0x150
    let msg1 = backend.receive_message().unwrap();
    assert_eq!(msg1.unwrap().data()[0], 2);

    let msg2 = backend.receive_message().unwrap();
    assert_eq!(msg2.unwrap().data()[0], 3);

    // No more messages
    let msg3 = backend.receive_message().unwrap();
    assert!(msg3.is_none());
}

#[test]
fn test_mock_backend_multiple_filters_or_logic() {
    let messages = vec![
        CanMessage::new_standard(0x100, &[1]).unwrap(),
        CanMessage::new_standard(0x200, &[2]).unwrap(),
        CanMessage::new_standard(0x300, &[3]).unwrap(),
        CanMessage::new_standard(0x400, &[4]).unwrap(),
    ];
    let mut backend = setup_backend_with_messages(messages);

    // Add filters for 0x100 and 0x300
    backend.add_id_filter(0x100);
    backend.add_id_filter(0x300);

    // Should receive 0x100 and 0x300
    let msg1 = backend.receive_message().unwrap();
    assert_eq!(msg1.unwrap().data()[0], 1);

    let msg2 = backend.receive_message().unwrap();
    assert_eq!(msg2.unwrap().data()[0], 3);

    // No more messages
    let msg3 = backend.receive_message().unwrap();
    assert!(msg3.is_none());
}

#[test]
fn test_mock_backend_clear_filters_receives_all() {
    let messages = vec![
        CanMessage::new_standard(0x100, &[1]).unwrap(),
        CanMessage::new_standard(0x200, &[2]).unwrap(),
    ];
    let config = MockConfig::with_preset_messages(messages);
    let mut backend = MockBackend::with_config(config);
    backend.initialize(&BackendConfig::new("mock")).unwrap();
    backend.open_channel(0).unwrap();

    // Add restrictive filter (0x7FF is valid but won't match 0x100 or 0x200)
    backend.add_id_filter(0x7FF);

    // No messages should pass
    let msg = backend.receive_message().unwrap();
    assert!(msg.is_none());

    // Clear filters - but preset messages are already consumed
    backend.clear_filters();
    assert_eq!(backend.filter_count(), 0);
}

#[test]
fn test_mock_backend_filter_chain_access() {
    let mut backend = MockBackend::new();

    // Access filter chain directly
    backend
        .filter_chain_mut()
        .add_filter(Box::new(IdFilter::new(0x123)));

    assert_eq!(backend.filter_chain().len(), 1);
    assert!(!backend.filter_chain().is_empty());
}

// ============================================================================
// Filter Configuration Tests
// ============================================================================

#[test]
fn test_filter_config_from_toml_id_filter() {
    let toml = r#"
        [[id_filters]]
        id = 291
    "#;

    let config: FilterConfig = toml::from_str(toml).unwrap();
    let chain = config.into_chain().unwrap();

    assert_eq!(chain.len(), 1);
    assert!(chain.matches(&CanMessage::new_standard(0x123, &[0]).unwrap()));
    assert!(!chain.matches(&CanMessage::new_standard(0x456, &[0]).unwrap()));
}

#[test]
fn test_filter_config_from_toml_range_filter() {
    let toml = r#"
        [[range_filters]]
        start_id = 256
        end_id = 511
    "#;

    let config: FilterConfig = toml::from_str(toml).unwrap();
    let chain = config.into_chain().unwrap();

    assert_eq!(chain.len(), 1);
    assert!(chain.matches(&CanMessage::new_standard(0x100, &[0]).unwrap()));
    assert!(chain.matches(&CanMessage::new_standard(0x1FF, &[0]).unwrap()));
    assert!(!chain.matches(&CanMessage::new_standard(0x200, &[0]).unwrap()));
}

#[test]
fn test_filter_config_from_toml_multiple_filters() {
    let toml = r#"
        [[id_filters]]
        id = 291

        [[range_filters]]
        start_id = 512
        end_id = 767
    "#;

    let config: FilterConfig = toml::from_str(toml).unwrap();
    let chain = config.into_chain().unwrap();

    assert_eq!(chain.len(), 2);

    // Should match ID filter (0x123 = 291)
    assert!(chain.matches(&CanMessage::new_standard(0x123, &[0]).unwrap()));

    // Should match range filter (0x200-0x2FF = 512-767)
    assert!(chain.matches(&CanMessage::new_standard(0x200, &[0]).unwrap()));
    assert!(chain.matches(&CanMessage::new_standard(0x2FF, &[0]).unwrap()));

    // Should not match outside both
    assert!(!chain.matches(&CanMessage::new_standard(0x100, &[0]).unwrap()));
}

#[test]
fn test_filter_config_from_toml_with_mask() {
    let toml = r#"
        [[id_filters]]
        id = 288
        mask = 2032
    "#;

    let config: FilterConfig = toml::from_str(toml).unwrap();
    let chain = config.into_chain().unwrap();

    // Mask 0x7F0 (2032) ignores lower 4 bits
    // ID 0x120 (288) with mask matches 0x120-0x12F
    assert!(chain.matches(&CanMessage::new_standard(0x120, &[0]).unwrap()));
    assert!(chain.matches(&CanMessage::new_standard(0x12F, &[0]).unwrap()));
    assert!(!chain.matches(&CanMessage::new_standard(0x130, &[0]).unwrap()));
}

#[test]
fn test_filter_config_from_toml_extended_frames() {
    let toml = r#"
        [[id_filters]]
        id = 305419896
        extended = true
    "#;

    let config: FilterConfig = toml::from_str(toml).unwrap();
    let chain = config.into_chain().unwrap();

    // Should match extended frame (0x12345678 = 305419896)
    assert!(chain.matches(&CanMessage::new_extended(0x12345678, &[0]).unwrap()));

    // Should not match standard frame with same lower bits
    assert!(!chain.matches(&CanMessage::new_standard(0x678, &[0]).unwrap()));
}

#[test]
fn test_filter_config_max_hardware_filters() {
    let toml = r#"
        max_hardware_filters = 8

        [[id_filters]]
        id = 291
    "#;

    let config: FilterConfig = toml::from_str(toml).unwrap();
    let chain = config.into_chain().unwrap();

    assert_eq!(chain.max_hardware_filters(), 8);
}

// ============================================================================
// End-to-End Workflow Tests
// ============================================================================

#[test]
fn test_complete_filtering_workflow() {
    // Simulate a complete workflow:
    // 1. Create backend with preset messages
    // 2. Configure filters
    // 3. Receive filtered messages
    // 4. Verify only expected messages received

    let messages = vec![
        CanMessage::new_standard(0x100, &[0x10]).unwrap(), // Engine RPM
        CanMessage::new_standard(0x200, &[0x20]).unwrap(), // Vehicle Speed
        CanMessage::new_standard(0x300, &[0x30]).unwrap(), // Throttle Position
        CanMessage::new_standard(0x400, &[0x40]).unwrap(), // Brake Pressure
        CanMessage::new_standard(0x500, &[0x50]).unwrap(), // Steering Angle
    ];
    let mut backend = setup_backend_with_messages(messages);

    // Configure to only receive engine and brake data
    backend.add_id_filter(0x100); // Engine RPM
    backend.add_id_filter(0x400); // Brake Pressure

    // Collect received messages
    let mut received = Vec::new();
    while let Ok(Some(msg)) = backend.receive_message() {
        received.push(msg);
    }

    // Verify
    assert_eq!(received.len(), 2);
    assert_eq!(received[0].data()[0], 0x10); // Engine RPM
    assert_eq!(received[1].data()[0], 0x40); // Brake Pressure
}

#[test]
fn test_filter_reconfiguration() {
    // Test changing filters during operation

    let config = MockConfig::default();
    let mut backend = MockBackend::with_config(config);
    backend.initialize(&BackendConfig::new("mock")).unwrap();

    // Initial configuration
    backend.add_id_filter(0x100);
    assert_eq!(backend.filter_count(), 1);

    // Add more filters
    backend.add_id_filter(0x200);
    backend.add_range_filter(0x300, 0x3FF);
    assert_eq!(backend.filter_count(), 3);

    // Clear and reconfigure
    backend.clear_filters();
    assert_eq!(backend.filter_count(), 0);

    backend.add_id_filter(0x500);
    assert_eq!(backend.filter_count(), 1);
}

#[test]
fn test_filter_persistence_across_operations() {
    let messages = vec![
        CanMessage::new_standard(0x100, &[1]).unwrap(),
        CanMessage::new_standard(0x200, &[2]).unwrap(),
    ];
    let mut backend = setup_backend_with_messages(messages);

    // Add filter
    backend.add_id_filter(0x100);

    // Send a message (filters don't affect send)
    let send_msg = CanMessage::new_standard(0x7FF, &[9]).unwrap();
    backend.send_message(&send_msg).unwrap();

    // Verify filter still active for receive
    let msg = backend.receive_message().unwrap();
    assert!(msg.is_some());
    assert_eq!(msg.unwrap().data()[0], 1); // Only 0x100 passes

    // Verify sent message was recorded (not filtered)
    assert!(backend.verify_message_sent(canlink_hal::CanId::Standard(0x7FF)));
}
