//! Connection monitoring example (T052)
//!
//! This example demonstrates:
//! - Creating a ConnectionMonitor with heartbeat interval
//! - Configuring auto-reconnect with exponential backoff
//! - Monitoring connection state changes
//! - Simulating disconnect/reconnect scenarios

use canlink_hal::monitor::{ConnectionMonitor, ConnectionState, ReconnectConfig};
use canlink_hal::{BackendConfig, CanBackend, CanMessage};
use canlink_mock::MockBackend;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== CANLink HAL - Connection Monitor Example ===\n");

    // Step 1: Create and initialize backend
    println!("1. Creating and initializing backend...");
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config)?;
    backend.open_channel(0)?;
    println!("   Backend initialized and channel opened\n");

    // Step 2: Create a basic connection monitor (no auto-reconnect)
    println!("2. Creating basic connection monitor...");
    let monitor = ConnectionMonitor::new(Duration::from_secs(1));
    println!("   Heartbeat interval: {:?}", monitor.heartbeat_interval());
    println!(
        "   Auto-reconnect enabled: {}",
        monitor.auto_reconnect_enabled()
    );
    println!("   Current state: {:?}\n", monitor.state());

    // Step 3: Create a monitor with auto-reconnect
    println!("3. Creating monitor with auto-reconnect...");
    let reconnect_config = ReconnectConfig::exponential_backoff(
        5,                          // max 5 retries
        Duration::from_millis(500), // start with 500ms
        2.0,                        // double each time
    );

    let monitor_with_reconnect =
        ConnectionMonitor::with_reconnect(Duration::from_secs(1), reconnect_config);

    println!(
        "   Auto-reconnect enabled: {}",
        monitor_with_reconnect.auto_reconnect_enabled()
    );
    if let Some(config) = monitor_with_reconnect.reconnect_config() {
        println!("   Max retries: {}", config.max_retries);
        println!("   Initial retry interval: {:?}", config.retry_interval);
        println!("   Backoff multiplier: {}", config.backoff_multiplier);
    }
    println!();

    // Step 4: Demonstrate reconnect interval calculation
    println!("4. Reconnect interval calculation (exponential backoff)...");
    let config = ReconnectConfig::exponential_backoff(5, Duration::from_secs(1), 2.0);
    for attempt in 0..5 {
        let interval = config.interval_for_attempt(attempt);
        println!("   Attempt {}: wait {:?}", attempt + 1, interval);
    }
    println!();

    // Step 5: Demonstrate fixed interval reconnect
    println!("5. Fixed interval reconnect configuration...");
    let fixed_config = ReconnectConfig::fixed_interval(3, Duration::from_secs(2));
    println!("   Max retries: {}", fixed_config.max_retries);
    for attempt in 0..3 {
        let interval = fixed_config.interval_for_attempt(attempt);
        println!("   Attempt {}: wait {:?}", attempt + 1, interval);
    }
    println!();

    // Step 6: Simulate disconnect/reconnect scenario
    println!("6. Simulating disconnect/reconnect scenario...");

    // Send a message (should succeed)
    let msg = CanMessage::new_standard(0x100, &[1, 2, 3, 4])?;
    backend.send_message(&msg)?;
    println!("   Message sent successfully");

    // Simulate disconnect
    backend.simulate_disconnect();
    println!("   Simulated hardware disconnect");
    println!("   Backend disconnected: {}", backend.is_disconnected());

    // Try to send (should fail)
    let result = backend.send_message(&msg);
    match result {
        Ok(_) => println!("   Message sent (unexpected)"),
        Err(e) => println!("   Send failed as expected: {}", e),
    }

    // Simulate reconnect
    backend.simulate_reconnect();
    println!("   Simulated hardware reconnect");
    println!("   Backend disconnected: {}", backend.is_disconnected());

    // Send again (should succeed)
    backend.send_message(&msg)?;
    println!("   Message sent successfully after reconnect\n");

    // Step 7: Demonstrate state transitions
    println!("7. Connection state transitions...");
    let mut monitor = ConnectionMonitor::new(Duration::from_secs(1));

    println!("   Initial state: {:?}", monitor.state());

    monitor.set_state(ConnectionState::Disconnected);
    println!("   After disconnect: {:?}", monitor.state());

    monitor.set_state(ConnectionState::Reconnecting);
    println!("   During reconnect: {:?}", monitor.state());

    monitor.set_state(ConnectionState::Connected);
    println!("   After reconnect: {:?}", monitor.state());
    println!();

    // Step 8: Check retry logic
    println!("8. Retry logic demonstration...");
    let config = ReconnectConfig {
        max_retries: 3,
        retry_interval: Duration::from_secs(1),
        backoff_multiplier: 2.0,
    };

    for attempt in 0..5 {
        let should_retry = config.should_retry(attempt);
        println!(
            "   Attempt {}: should_retry = {}",
            attempt + 1,
            should_retry
        );
    }
    println!();

    // Step 9: Unlimited retries configuration
    println!("9. Unlimited retries configuration...");
    let unlimited_config = ReconnectConfig {
        max_retries: 0, // 0 means unlimited
        retry_interval: Duration::from_secs(5),
        backoff_multiplier: 1.0,
    };
    println!(
        "   Max retries: {} (0 = unlimited)",
        unlimited_config.max_retries
    );
    println!(
        "   Should retry at attempt 100: {}",
        unlimited_config.should_retry(100)
    );
    println!(
        "   Should retry at attempt 1000: {}",
        unlimited_config.should_retry(1000)
    );
    println!();

    // Step 10: Cleanup
    println!("10. Cleaning up...");
    backend.close_channel(0)?;
    backend.close()?;
    println!("   Backend closed\n");

    println!("=== Example completed successfully! ===");
    Ok(())
}
