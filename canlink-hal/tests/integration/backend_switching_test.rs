//! Backend switching integration tests.
//!
//! These tests verify that the application can switch between different backends
//! at runtime without modifying business logic code.

use canlink_hal::{BackendConfig, BackendRegistry, CanBackend, CanMessage};
use canlink_mock::{MockBackend, MockBackendFactory, MockConfig};
use std::sync::Arc;

/// Test registering and creating backends from registry.
#[test]
fn test_registry_backend_creation() {
    let registry = BackendRegistry::new();
    let factory = Arc::new(MockBackendFactory::new());

    // Register backend
    assert!(registry.register(factory).is_ok());

    // Create backend from registry
    let config = BackendConfig::new("mock");
    let backend = registry.create("mock", &config);
    assert!(backend.is_ok());

    let backend = backend.unwrap();
    assert_eq!(backend.name(), "mock");
}

/// Test that the same application code works with different backend instances.
#[test]
fn test_application_code_backend_independence() {
    // Create two different backend instances
    let config1 = MockConfig::default();
    let config2 = MockConfig::can20_only();

    let mut backend1 = MockBackend::with_config(config1);
    let mut backend2 = MockBackend::with_config(config2);

    // Use the same application code with both backends
    assert!(use_backend(&mut backend1).is_ok());
    assert!(use_backend(&mut backend2).is_ok());
}

/// Test switching between backends at runtime.
#[test]
fn test_runtime_backend_switching() {
    let registry = BackendRegistry::new();
    let factory = Arc::new(MockBackendFactory::new());
    registry.register(factory).unwrap();

    let config = BackendConfig::new("mock");

    // Create and use first backend
    let mut backend1 = registry.create("mock", &config).unwrap();
    backend1.initialize(&config).unwrap();
    backend1.open_channel(0).unwrap();

    let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    backend1.send_message(&msg).unwrap();

    backend1.close_channel(0).unwrap();
    backend1.close().unwrap();

    // Create and use second backend (simulating switch)
    let mut backend2 = registry.create("mock", &config).unwrap();
    backend2.initialize(&config).unwrap();
    backend2.open_channel(0).unwrap();

    backend2.send_message(&msg).unwrap();

    backend2.close_channel(0).unwrap();
    backend2.close().unwrap();

    // Both backends should work identically
}

/// Test that registry correctly lists registered backends.
#[test]
fn test_registry_list_backends() {
    let registry = BackendRegistry::new();

    // Initially empty
    assert_eq!(registry.list_backends().len(), 0);

    // Register backend
    let factory = Arc::new(MockBackendFactory::new());
    registry.register(factory).unwrap();

    // Should list one backend
    let backends = registry.list_backends();
    assert_eq!(backends.len(), 1);
    assert!(backends.contains(&"mock".to_string()));
}

/// Test that registry prevents duplicate registration.
#[test]
fn test_registry_duplicate_prevention() {
    let registry = BackendRegistry::new();
    let factory1 = Arc::new(MockBackendFactory::new());
    let factory2 = Arc::new(MockBackendFactory::new());

    // First registration should succeed
    assert!(registry.register(factory1).is_ok());

    // Second registration with same name should fail
    let result = registry.register(factory2);
    assert!(result.is_err());
}

/// Test that registry can unregister backends.
#[test]
fn test_registry_unregister() {
    let registry = BackendRegistry::new();
    let factory = Arc::new(MockBackendFactory::new());

    registry.register(factory).unwrap();
    assert!(registry.is_registered("mock"));

    // Unregister should succeed
    assert!(registry.unregister("mock").is_ok());
    assert!(!registry.is_registered("mock"));

    // Unregistering again should fail
    assert!(registry.unregister("mock").is_err());
}

/// Test that registry provides backend information.
#[test]
fn test_registry_backend_info() {
    let registry = BackendRegistry::new();
    let factory = Arc::new(MockBackendFactory::new());
    registry.register(factory).unwrap();

    let info = registry.get_backend_info("mock");
    assert!(info.is_ok());

    let info = info.unwrap();
    assert_eq!(info.name, "mock");
    assert_eq!(info.version.major(), 0);
}

/// Test that registry fails gracefully for non-existent backends.
#[test]
fn test_registry_nonexistent_backend() {
    let registry = BackendRegistry::new();

    // Creating non-existent backend should fail
    let config = BackendConfig::new("nonexistent");
    let result = registry.create("nonexistent", &config);
    assert!(result.is_err());

    // Getting info for non-existent backend should fail
    let result = registry.get_backend_info("nonexistent");
    assert!(result.is_err());

    // is_registered should return false
    assert!(!registry.is_registered("nonexistent"));
}

/// Test global registry singleton.
#[test]
fn test_global_registry_singleton() {
    let registry1 = BackendRegistry::global();
    let registry2 = BackendRegistry::global();

    // Should be the same instance
    assert!(Arc::ptr_eq(&registry1, &registry2));
}

/// Test that backends created from registry work correctly.
#[test]
fn test_registry_created_backend_functionality() {
    let registry = BackendRegistry::new();
    let factory = Arc::new(MockBackendFactory::new());
    registry.register(factory).unwrap();

    let config = BackendConfig::new("mock");
    let mut backend = registry.create("mock", &config).unwrap();

    // Initialize
    assert!(backend.initialize(&config).is_ok());

    // Query capability
    let capability = backend.get_capability();
    assert!(capability.is_ok());

    // Open channel
    assert!(backend.open_channel(0).is_ok());

    // Send message
    let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    assert!(backend.send_message(&msg).is_ok());

    // Close
    assert!(backend.close_channel(0).is_ok());
    assert!(backend.close().is_ok());
}

/// Test backend switching with different configurations.
#[test]
fn test_backend_switching_with_different_configs() {
    let registry = BackendRegistry::new();

    // Register factory with default config
    let factory1 = Arc::new(MockBackendFactory::new());
    registry.register(factory1).unwrap();

    // Create backend with default config
    let config = BackendConfig::new("mock");
    let mut backend1 = registry.create("mock", &config).unwrap();
    backend1.initialize(&config).unwrap();

    let cap1 = backend1.get_capability().unwrap();
    assert!(cap1.supports_canfd);

    backend1.close().unwrap();

    // In a real scenario, you would register a different backend type here
    // For this test, we're demonstrating the pattern
}

/// Test that application logic is truly backend-agnostic.
#[test]
fn test_backend_agnostic_application_logic() {
    // This test demonstrates that the same function can work with any backend
    let mut backend1 = MockBackend::new();
    let mut backend2 = MockBackend::with_config(MockConfig::can20_only());

    // Same function works with both
    assert!(send_test_message(&mut backend1).is_ok());
    assert!(send_test_message(&mut backend2).is_ok());
}

/// Test concurrent backend usage (different instances).
#[test]
fn test_multiple_backend_instances() {
    let registry = BackendRegistry::new();
    let factory = Arc::new(MockBackendFactory::new());
    registry.register(factory).unwrap();

    let config = BackendConfig::new("mock");

    // Create multiple backend instances
    let mut backend1 = registry.create("mock", &config).unwrap();
    let mut backend2 = registry.create("mock", &config).unwrap();

    // Both should work independently
    backend1.initialize(&config).unwrap();
    backend2.initialize(&config).unwrap();

    backend1.open_channel(0).unwrap();
    backend2.open_channel(0).unwrap();

    let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    assert!(backend1.send_message(&msg).is_ok());
    assert!(backend2.send_message(&msg).is_ok());

    backend1.close().unwrap();
    backend2.close().unwrap();
}

// Helper functions

/// Generic application code that works with any backend.
fn use_backend(backend: &mut dyn CanBackend) -> Result<(), Box<dyn std::error::Error>> {
    let config = BackendConfig::new(backend.name());
    backend.initialize(&config)?;
    backend.open_channel(0)?;

    let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4])?;
    backend.send_message(&msg)?;

    backend.close_channel(0)?;
    backend.close()?;
    Ok(())
}

/// Another example of backend-agnostic code.
fn send_test_message(backend: &mut dyn CanBackend) -> Result<(), Box<dyn std::error::Error>> {
    let config = BackendConfig::new(backend.name());
    backend.initialize(&config)?;
    backend.open_channel(0)?;

    let msg = CanMessage::new_standard(0x456, &[0xAA, 0xBB, 0xCC])?;
    backend.send_message(&msg)?;

    backend.close_channel(0)?;
    backend.close()?;
    Ok(())
}
