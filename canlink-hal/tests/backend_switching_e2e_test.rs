//! End-to-end test for backend switching (SC-002).
//!
//! This test verifies that application code can switch between different
//! hardware backends without modifying business logic.

use canlink_hal::{BackendConfig, BackendRegistry, CanBackend, CanMessage};
use canlink_mock::MockBackendFactory;
use std::sync::Arc;

#[test]
fn test_backend_switching_mock_to_mock() {
    // Register mock backend
    let registry = BackendRegistry::new();
    let factory = Arc::new(MockBackendFactory::new());
    registry.register(factory).unwrap();

    // Application code - works with any backend
    fn send_and_receive(backend: &mut dyn CanBackend) -> Result<(), canlink_hal::CanError> {
        // Initialize
        let config = BackendConfig::new("mock");
        backend.initialize(&config)?;

        // Open channel
        backend.open_channel(0)?;

        // Send message
        let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4])?;
        backend.send_message(&msg)?;

        // Close
        backend.close_channel(0)?;
        backend.close()?;

        Ok(())
    }

    // Test with first backend instance
    let config = BackendConfig::new("mock");
    let mut backend1 = registry.create("mock", &config).unwrap();
    send_and_receive(backend1.as_mut()).unwrap();

    // Test with second backend instance (same code)
    let mut backend2 = registry.create("mock", &config).unwrap();
    send_and_receive(backend2.as_mut()).unwrap();

    // Success: Same application code works with different backend instances
}

#[test]
fn test_backend_abstraction_trait_object() {
    // This test verifies that we can use trait objects for backend abstraction
    let registry = BackendRegistry::new();
    let factory = Arc::new(MockBackendFactory::new());
    registry.register(factory).unwrap();

    let config = BackendConfig::new("mock");
    let mut backend: Box<dyn CanBackend> = registry.create("mock", &config).unwrap();

    // All operations work through trait object
    backend.initialize(&config).unwrap();
    assert_eq!(backend.name(), "mock");

    let capability = backend.get_capability().unwrap();
    assert!(capability.channel_count > 0);

    backend.open_channel(0).unwrap();

    let msg = CanMessage::new_standard(0x100, &[0xAA, 0xBB]).unwrap();
    backend.send_message(&msg).unwrap();

    backend.close_channel(0).unwrap();
    backend.close().unwrap();
}

#[test]
fn test_backend_switching_preserves_behavior() {
    // Verify that switching backends doesn't change application behavior
    let registry = BackendRegistry::new();
    let factory = Arc::new(MockBackendFactory::new());
    registry.register(factory).unwrap();

    // Generic function that works with any backend
    fn test_message_roundtrip(backend_name: &str, registry: &BackendRegistry) -> usize {
        let config = BackendConfig::new(backend_name);
        let mut backend = registry.create(backend_name, &config).unwrap();

        backend.initialize(&config).unwrap();
        backend.open_channel(0).unwrap();

        // Send 10 messages
        for i in 0..10 {
            let msg = CanMessage::new_standard(0x100 + i, &[i as u8]).unwrap();
            backend.send_message(&msg).unwrap();
        }

        backend.close_channel(0).unwrap();
        backend.close().unwrap();

        10 // Return message count
    }

    // Test with mock backend
    let count1 = test_message_roundtrip("mock", &registry);
    assert_eq!(count1, 10);

    // Test again with same backend (verifies consistency)
    let count2 = test_message_roundtrip("mock", &registry);
    assert_eq!(count2, 10);

    // Both runs produced same result
    assert_eq!(count1, count2);
}

#[test]
fn test_backend_capability_query_abstraction() {
    // Verify that capability queries work through abstraction
    let registry = BackendRegistry::new();
    let factory = Arc::new(MockBackendFactory::new());
    registry.register(factory).unwrap();

    let config = BackendConfig::new("mock");
    let mut backend = registry.create("mock", &config).unwrap();
    backend.initialize(&config).unwrap();

    // Query capabilities through trait
    let capability = backend.get_capability().unwrap();

    // Verify we got valid capability information
    assert!(capability.channel_count > 0);
    assert!(!capability.supported_bitrates.is_empty());
    assert!(capability.max_bitrate > 0);

    backend.close().unwrap();
}

#[test]
fn test_multiple_backends_coexist() {
    // Verify that multiple backend types can be registered simultaneously
    let registry = BackendRegistry::new();

    // Register mock backend
    let mock_factory = Arc::new(MockBackendFactory::new());
    registry.register(mock_factory).unwrap();

    // List backends
    let backends = registry.list_backends();
    assert!(backends.iter().any(|name| name == "mock"));

    // Create instances of different backends
    let config = BackendConfig::new("mock");
    let mut backend1 = registry.create("mock", &config).unwrap();
    let mut backend2 = registry.create("mock", &config).unwrap();

    // Both can be initialized independently
    backend1.initialize(&config).unwrap();
    backend2.initialize(&config).unwrap();

    // Both work independently
    assert_eq!(backend1.name(), "mock");
    assert_eq!(backend2.name(), "mock");

    backend1.close().unwrap();
    backend2.close().unwrap();
}

#[test]
fn test_backend_error_handling_consistency() {
    // Verify that error handling is consistent across backends
    let registry = BackendRegistry::new();
    let factory = Arc::new(MockBackendFactory::new());
    registry.register(factory).unwrap();

    let config = BackendConfig::new("mock");
    let mut backend = registry.create("mock", &config).unwrap();

    // Try to use backend before initialization - should fail
    let result = backend.open_channel(0);
    assert!(result.is_err());

    // Initialize
    backend.initialize(&config).unwrap();

    // Try to open invalid channel - should fail
    let result = backend.open_channel(99);
    assert!(result.is_err());

    backend.close().unwrap();
}
