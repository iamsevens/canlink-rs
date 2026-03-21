//! Configuration hot-reload example (T054)
//!
//! This example demonstrates:
//! - Creating a ConfigWatcher to monitor configuration files
//! - Handling configuration change events
//! - Applying configuration changes at runtime
//!
//! Note: This example requires the `hot-reload` feature to be enabled.
//! Run with: cargo run --example hot_reload --features hot-reload

use canlink_hal::{BackendConfig, CanBackend};
use canlink_mock::MockBackend;
use std::fs;

#[cfg(feature = "hot-reload")]
use canlink_hal::hot_reload::ConfigWatcher;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== CANLink HAL - Hot Reload Example ===\n");

    #[cfg(not(feature = "hot-reload"))]
    {
        println!("This example requires the 'hot-reload' feature.");
        println!("Run with: cargo run --example hot_reload --features hot-reload\n");

        // Demonstrate what hot-reload would do conceptually
        demonstrate_config_reload_concept()?;
    }

    #[cfg(feature = "hot-reload")]
    {
        demonstrate_hot_reload()?;
    }

    println!("=== Example completed successfully! ===");
    Ok(())
}

/// Demonstrates the concept of configuration reload without the hot-reload feature
#[cfg(not(feature = "hot-reload"))]
fn demonstrate_config_reload_concept() -> Result<(), Box<dyn std::error::Error>> {
    println!("Demonstrating configuration reload concept...\n");

    // Step 1: Create initial configuration
    println!("1. Initial Configuration");
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config)?;
    backend.open_channel(0)?;
    println!("   Backend initialized with default config\n");

    // Step 2: Show what a config file might look like
    println!("2. Example Configuration File (config.toml)");
    let example_config = r#"
# CANLink Configuration File
[backend]
name = "mock"
channel = 0

[queue]
capacity = 1000
overflow_policy = "drop_oldest"

[filter]
enabled = true
ids = [0x100, 0x200, 0x300]

[monitor]
heartbeat_interval_ms = 1000
auto_reconnect = true
max_retries = 5
"#;
    println!("{}", example_config);

    // Step 3: Simulate configuration change
    println!("3. Simulating Configuration Change");
    println!("   In a real scenario, the ConfigWatcher would detect file changes");
    println!("   and trigger a callback to reload the configuration.\n");

    // Step 4: Show how to apply new configuration
    println!("4. Applying New Configuration");

    // Simulate reading new config values
    let new_filter_ids = vec![0x100, 0x200, 0x300, 0x400];
    println!("   New filter IDs: {:?}", new_filter_ids);

    // Apply filters
    for id in &new_filter_ids {
        backend.add_id_filter(*id);
    }
    println!("   Applied {} filters to backend", backend.filter_count());
    println!();

    // Step 5: Demonstrate filter update
    println!("5. Updating Filters at Runtime");
    backend.clear_filters();
    println!("   Cleared existing filters");

    let updated_filter_ids = vec![0x500, 0x600];
    for id in &updated_filter_ids {
        backend.add_id_filter(*id);
    }
    println!("   Applied new filters: {:?}", updated_filter_ids);
    println!("   Current filter count: {}", backend.filter_count());
    println!();

    // Step 6: Cleanup
    println!("6. Cleanup");
    backend.close_channel(0)?;
    backend.close()?;
    println!("   Backend closed\n");

    Ok(())
}

#[cfg(feature = "hot-reload")]
fn demonstrate_hot_reload() -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    println!("Hot-reload feature is enabled!\n");

    // Step 1: Create a temporary config file
    println!("1. Creating temporary configuration file...");
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("canlink_example_config.toml");

    let initial_config = r#"
[backend]
name = "mock"

[queue]
capacity = 500
"#;
    fs::write(&config_path, initial_config)?;
    println!("   Config file created at: {}", config_path.display());
    println!();

    // Step 2: Create backend
    println!("2. Creating and initializing backend...");
    let backend = Arc::new(Mutex::new(MockBackend::new()));
    {
        let mut b = backend.lock().unwrap();
        let config = BackendConfig::new("mock");
        b.initialize(&config)?;
        b.open_channel(0)?;
    }
    println!("   Backend initialized\n");

    // Step 3: Create ConfigWatcher
    println!("3. Creating ConfigWatcher...");
    let mut watcher = ConfigWatcher::new(&config_path)?;

    // Track if config changed
    let config_changed = Arc::new(Mutex::new(false));
    let config_changed_clone = Arc::clone(&config_changed);

    // Register callback
    watcher.on_config_change(move |path: &std::path::Path| {
        println!(
            "   [Callback] Configuration file changed: {}",
            path.display()
        );
        *config_changed_clone.lock().unwrap() = true;
    });

    // Start watching
    watcher.start()?;
    println!("   ConfigWatcher started\n");

    // Step 4: Modify the config file
    println!("4. Modifying configuration file...");
    let updated_config = r#"
[backend]
name = "mock"

[queue]
capacity = 1000

[filter]
ids = [0x100, 0x200]
"#;

    // Small delay to ensure watcher is ready
    thread::sleep(Duration::from_millis(100));

    fs::write(&config_path, updated_config)?;
    println!("   Config file updated\n");

    // Step 5: Wait for change detection
    println!("5. Waiting for change detection...");
    thread::sleep(Duration::from_secs(2));

    if *config_changed.lock().unwrap() {
        println!("   Configuration change detected!\n");
    } else {
        println!("   (Change detection may take a moment...)\n");
    }

    // Step 6: Stop watcher and cleanup
    println!("6. Stopping ConfigWatcher...");
    watcher.stop();
    println!("   ConfigWatcher stopped\n");

    // Cleanup
    println!("7. Cleanup...");
    {
        let mut b = backend.lock().unwrap();
        b.close_channel(0)?;
        b.close()?;
    }

    // Remove temp file
    if config_path.exists() {
        fs::remove_file(&config_path)?;
        println!("   Temporary config file removed");
    }
    println!("   Backend closed\n");

    Ok(())
}
