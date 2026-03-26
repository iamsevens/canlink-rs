//! Filter Configuration Example (T033)
//!
//! This example demonstrates how to use the canlink-hal filter module
//! and load filter configurations from TOML files.
//!
//! ## Topics Covered
//!
//! - Using IdFilter for exact ID matching
//! - Using RangeFilter for ID range matching
//! - Using FilterChain to combine multiple filters
//! - Loading filter configuration from TOML
//! - Integrating filters with MockBackend
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run --example filter_config
//! ```

use canlink_hal::filter::{FilterChain, FilterConfig, IdFilter, RangeFilter};
use canlink_hal::message::CanMessage;
use canlink_hal::{BackendConfig, CanBackend};
use canlink_mock::{MockBackend, MockConfig};

/// Scenario 1: Basic filter usage with IdFilter and RangeFilter
fn scenario_basic_filters() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Scenario 1: Basic Filter Usage ===\n");

    // Create a filter chain with max 4 hardware filters
    let mut chain = FilterChain::new(4);

    // Add an exact ID filter for 0x123
    chain.add_filter(Box::new(IdFilter::new(0x123)));
    println!("Added IdFilter for 0x123");

    // Add a range filter for 0x200-0x2FF
    chain.add_filter(Box::new(RangeFilter::new(0x200, 0x2FF)));
    println!("Added RangeFilter for 0x200-0x2FF");

    // Add a mask filter for 0x300-0x30F (mask ignores lower 4 bits)
    chain.add_filter(Box::new(IdFilter::with_mask(0x300, 0x7F0)));
    println!("Added IdFilter with mask 0x7F0 for 0x300-0x30F");

    println!("\nFilter chain stats:");
    println!("  Total filters: {}", chain.len());
    println!("  Hardware filters: {}", chain.hardware_filter_count());
    println!("  Software filters: {}", chain.software_filter_count());

    // Test messages
    let test_messages = vec![
        (0x123, "Exact match 0x123"),
        (0x124, "Near 0x123 but different"),
        (0x200, "Start of range"),
        (0x250, "Middle of range"),
        (0x2FF, "End of range"),
        (0x300, "Outside range, but matches mask"),
        (0x305, "Matches mask filter"),
        (0x310, "Outside mask range"),
        (0x400, "No match"),
    ];

    println!("\nTesting messages:");
    for (id, description) in test_messages {
        let msg = CanMessage::new_standard(id, &[0u8; 8])?;
        let matches = chain.matches(&msg);
        println!(
            "  0x{:03X} - {} - {}",
            id,
            description,
            if matches { "PASS" } else { "BLOCKED" }
        );
    }

    Ok(())
}

/// Scenario 2: Loading filters from TOML configuration
fn scenario_toml_config() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n\n=== Scenario 2: TOML Configuration ===\n");

    // Example TOML configuration
    let toml_config = r#"
# Maximum hardware filters (optional, default: 4)
max_hardware_filters = 8

# ID filters - exact match or mask-based
[[id_filters]]
id = 291  # 0x123 in decimal

[[id_filters]]
id = 1024  # 0x400
mask = 1792  # 0x700 - matches 0x400-0x4FF

[[id_filters]]
id = 305419896  # 0x12345678 - extended frame
extended = true

# Range filters
[[range_filters]]
start_id = 512  # 0x200
end_id = 767    # 0x2FF

[[range_filters]]
start_id = 65536   # 0x10000 - extended frame range
end_id = 131071    # 0x1FFFF
extended = true
"#;

    println!("TOML Configuration:\n{}", toml_config);

    // Parse the configuration
    let config: FilterConfig = toml::from_str(toml_config)?;
    println!("Parsed configuration successfully!");

    // Build the filter chain
    let chain = config.into_chain()?;

    println!("\nFilter chain stats:");
    println!("  Total filters: {}", chain.len());
    println!("  Max hardware filters: {}", chain.max_hardware_filters());
    println!(
        "  Hardware filters in use: {}",
        chain.hardware_filter_count()
    );

    // Test standard frame messages
    println!("\nTesting standard frame messages:");
    let standard_tests = vec![
        (0x123u16, "ID filter exact match"),
        (0x200, "Range filter start"),
        (0x2FF, "Range filter end"),
        (0x400, "Mask filter base"),
        (0x4FF, "Mask filter end"),
        (0x500, "Outside all filters"),
    ];

    for (id, description) in standard_tests {
        let msg = CanMessage::new_standard(id, &[0u8; 8])?;
        let matches = chain.matches(&msg);
        println!(
            "  0x{:03X} - {} - {}",
            id,
            description,
            if matches { "PASS" } else { "BLOCKED" }
        );
    }

    // Test extended frame messages
    println!("\nTesting extended frame messages:");
    let extended_tests = vec![
        (0x12345678u32, "Extended ID filter exact match"),
        (0x10000, "Extended range filter start"),
        (0x15000, "Extended range filter middle"),
        (0x1FFFF, "Extended range filter end"),
        (0x20000, "Outside extended range"),
    ];

    for (id, description) in extended_tests {
        let msg = CanMessage::new_extended(id, &[0u8; 8])?;
        let matches = chain.matches(&msg);
        println!(
            "  0x{:08X} - {} - {}",
            id,
            description,
            if matches { "PASS" } else { "BLOCKED" }
        );
    }

    Ok(())
}

/// Scenario 3: Integrating filters with MockBackend
fn scenario_backend_integration() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n\n=== Scenario 3: Backend Integration ===\n");

    // Create preset messages simulating CAN bus traffic
    let preset_messages = vec![
        CanMessage::new_standard(0x100, &[0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])?, // Engine RPM
        CanMessage::new_standard(0x200, &[0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])?, // Vehicle Speed
        CanMessage::new_standard(0x300, &[0x30, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])?, // Throttle
        CanMessage::new_standard(0x400, &[0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])?, // Brake
        CanMessage::new_standard(0x500, &[0x50, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])?, // Steering
        CanMessage::new_standard(0x600, &[0x60, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])?, // Transmission
        CanMessage::new_standard(0x700, &[0x70, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])?, // Diagnostics
    ];

    println!("Created {} preset messages", preset_messages.len());

    // Create and initialize backend
    let mock_config = MockConfig::with_preset_messages(preset_messages);
    let mut backend = MockBackend::with_config(mock_config);
    backend.initialize(&BackendConfig::new("mock"))?;
    backend.open_channel(0)?;

    // Configure filters - only receive engine, brake, and diagnostics
    println!("\nConfiguring filters:");
    println!("  - Engine RPM (0x100)");
    println!("  - Brake (0x400)");
    println!("  - Diagnostics (0x700)");

    backend.add_id_filter(0x100); // Engine RPM
    backend.add_id_filter(0x400); // Brake
    backend.add_id_filter(0x700); // Diagnostics

    println!("\nFilter count: {}", backend.filter_count());

    // Receive filtered messages
    println!("\nReceiving filtered messages:");
    let mut received_count = 0;
    while let Some(msg) = backend.receive_message()? {
        received_count += 1;
        println!(
            "  Received: ID=0x{:03X}, Data[0]=0x{:02X}",
            match msg.id() {
                canlink_hal::CanId::Standard(id) => id as u32,
                canlink_hal::CanId::Extended(id) => id,
            },
            msg.data()[0]
        );
    }

    println!(
        "\nTotal messages received: {} (out of 7 preset)",
        received_count
    );
    println!("Messages filtered out: {}", 7 - received_count);

    // Clear filters and demonstrate pass-through
    println!("\n--- Clearing filters ---");
    backend.clear_filters();
    println!("Filter count after clear: {}", backend.filter_count());

    backend.close()?;

    Ok(())
}

/// Scenario 4: Dynamic filter reconfiguration
fn scenario_dynamic_reconfiguration() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n\n=== Scenario 4: Dynamic Reconfiguration ===\n");

    let mut backend = MockBackend::new();
    backend.initialize(&BackendConfig::new("mock"))?;

    println!("Initial state: {} filters", backend.filter_count());

    // Phase 1: Monitor engine data
    println!("\nPhase 1: Monitoring engine data");
    backend.add_id_filter(0x100); // Engine RPM
    backend.add_id_filter(0x101); // Engine Temp
    backend.add_id_filter(0x102); // Engine Load
    println!("  Filters: {}", backend.filter_count());

    // Phase 2: Switch to chassis monitoring
    println!("\nPhase 2: Switching to chassis monitoring");
    backend.clear_filters();
    backend.add_range_filter(0x200, 0x2FF); // All chassis messages
    println!("  Filters: {}", backend.filter_count());

    // Phase 3: Add diagnostics
    println!("\nPhase 3: Adding diagnostics");
    backend.add_id_filter(0x7DF); // OBD-II broadcast
    backend.add_range_filter(0x7E0, 0x7EF); // OBD-II responses
    println!("  Filters: {}", backend.filter_count());

    // Check filter chain details
    let chain = backend.filter_chain();
    println!("\nFinal filter chain:");
    println!("  Total filters: {}", chain.len());
    println!("  Hardware filters: {}", chain.hardware_filter_count());
    println!("  Has hardware capacity: {}", chain.has_hardware_capacity());

    backend.close()?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Filter Configuration Example ===\n");
    println!("This example demonstrates the canlink-hal filter module.\n");

    scenario_basic_filters()?;
    scenario_toml_config()?;
    scenario_backend_integration()?;
    scenario_dynamic_reconfiguration()?;

    println!("\n=== All scenarios completed ===");

    Ok(())
}
