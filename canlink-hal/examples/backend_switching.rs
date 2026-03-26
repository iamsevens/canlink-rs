//! Backend switching example for `CANLink` HAL.
//!
//! This example demonstrates:
//! - Registering multiple backends with the global registry
//! - Listing available backends
//! - Querying backend information
//! - Creating backends from the registry
//! - Switching between backends at runtime
//! - Using the same application code with different backends
//!
//! Run with:
//! `cargo run -p canlink-hal --example backend_switching --features "canlink-hal/isotp canlink-hal/periodic"`

use canlink_hal::{BackendConfig, BackendRegistry, CanBackend, CanMessage};
use canlink_mock::{MockBackendFactory, MockConfig};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== CANLink HAL - Backend Switching Example ===\n");

    // Step 1: Get the global backend registry
    println!("1. Getting global backend registry...");
    let registry = BackendRegistry::global();
    println!("   ✓ Registry obtained\n");

    // Step 2: Register Mock backend
    println!("2. Registering Mock backend...");
    let mock_factory = Arc::new(MockBackendFactory::new());
    registry.register(mock_factory)?;
    println!("   ✓ Mock backend registered\n");

    // Step 3: Register a second Mock backend with different config (simulating different hardware)
    println!("3. Registering Mock CAN 2.0 backend...");
    let _mock_can20_factory = Arc::new(MockBackendFactory::with_config(MockConfig::can20_only()));
    // Note: This would fail because "mock" is already registered
    // In a real scenario, you'd register different backend types (e.g., "tsmaster", "peak", "kvaser")
    println!("   ℹ Skipping (same name as first backend)\n");

    // Step 4: List all available backends
    println!("4. Listing available backends...");
    let backends = registry.list_backends();
    println!("   ✓ Available backends: {:?}", backends);
    println!("   ✓ Total: {} backend(s)\n", backends.len());

    // Step 5: Query backend information
    println!("5. Querying backend information...");
    for backend_name in &backends {
        let info = registry.get_backend_info(backend_name)?;
        println!("   ✓ Backend: {}", info.name);
        println!("     - Version: {}", info.version);
    }
    println!();

    // Step 6: Create backend instance from registry
    println!("6. Creating backend from registry...");
    let config = BackendConfig::new("mock");
    let mut backend = registry.create("mock", &config)?;
    println!("   ✓ Backend created: {}", backend.name());
    println!("   ✓ Version: {}\n", backend.version());

    // Step 7: Use the backend (same code works for any backend)
    println!("7. Using the backend...");
    use_backend(&mut *backend)?;
    println!();

    // Step 8: Demonstrate backend switching
    println!("8. Demonstrating backend switching...");
    println!("   ℹ In a real application, you would:");
    println!("   - Load backend name from config file (canlink.toml)");
    println!("   - Create backend using registry.create(name, config)");
    println!("   - Application code remains unchanged");
    println!();

    // Step 9: Simulate switching to different backend
    println!("9. Simulating backend switch...");
    backend.close()?;
    println!("   ✓ Closed first backend");

    // Create a new backend instance (in real app, this would be a different backend type)
    let mut backend2 = registry.create("mock", &config)?;
    println!("   ✓ Created second backend: {}", backend2.name());

    // Use the same application code
    use_backend(&mut *backend2)?;
    backend2.close()?;
    println!("   ✓ Closed second backend\n");

    // Step 10: Check if backend is registered
    println!("10. Checking backend registration...");
    if registry.is_registered("mock") {
        println!("   ✓ 'mock' backend is registered");
    }
    if !registry.is_registered("nonexistent") {
        println!("   ✓ 'nonexistent' backend is not registered");
    }
    println!();

    println!("=== Example completed successfully! ===");
    println!("\nKey Takeaways:");
    println!("- Application code is independent of backend implementation");
    println!("- Backends can be switched by changing configuration");
    println!("- Registry provides runtime backend discovery");
    println!("- Same CanBackend trait works for all hardware types");

    Ok(())
}

/// Application code that works with any backend.
///
/// This function demonstrates that the same code can work with any
/// backend implementation (Mock, `TSMaster`, PEAK, etc.) without modification.
fn use_backend(backend: &mut dyn CanBackend) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize
    let config = BackendConfig::new(backend.name());
    backend.initialize(&config)?;
    println!("   ✓ Initialized backend: {}", backend.name());

    // Query capabilities
    let capability = backend.get_capability()?;
    println!(
        "   ✓ Capabilities: {} channels, CAN-FD: {}",
        capability.channel_count, capability.supports_canfd
    );

    // Open channel
    backend.open_channel(0)?;
    println!("   ✓ Opened channel 0");

    // Send a message
    let msg = CanMessage::new_standard(0x123, &[0x01, 0x02, 0x03, 0x04])?;
    backend.send_message(&msg)?;
    println!("   ✓ Sent message: ID={:?}", msg.id());

    // Try to receive (may return None if no messages available)
    if let Some(received) = backend.receive_message()? {
        println!("   ✓ Received message: ID={:?}", received.id());
    } else {
        println!("   ℹ No messages available to receive");
    }

    // Close channel
    backend.close_channel(0)?;
    println!("   ✓ Closed channel 0");

    Ok(())
}
