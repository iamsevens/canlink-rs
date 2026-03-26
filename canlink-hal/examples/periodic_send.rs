//! Periodic message sending example.
//!
//! This example demonstrates:
//! - Creating a periodic message scheduler
//! - Adding multiple periodic messages with different intervals
//! - Dynamic data and interval updates
//! - Monitoring send statistics
//! - Graceful shutdown
//!
//! Run with: `cargo run -p canlink-hal --example periodic_send --features "canlink-hal/periodic"`

use canlink_hal::periodic::{run_scheduler, PeriodicMessage, PeriodicScheduler};
use canlink_hal::{BackendConfig, CanBackend, CanMessage};
use canlink_mock::MockBackend;
use std::time::Duration;
use tokio::task::LocalSet;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== CANLink HAL - Periodic Message Sending Example ===\n");

    // Use LocalSet because MockBackend is not Send
    let local = LocalSet::new();

    local
        .run_until(async {
            run_example().await.expect("Example failed");
        })
        .await;

    println!("\n=== Example completed successfully! ===");
    Ok(())
}

async fn run_example() -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Create and initialize backend
    println!("1. Creating and initializing Mock backend...");
    let mut backend = MockBackend::new();
    backend.initialize(&BackendConfig::new("mock"))?;
    backend.open_channel(0)?;
    println!("   ✓ Backend initialized\n");

    // Step 2: Create periodic scheduler
    println!("2. Creating periodic scheduler...");
    let (scheduler, command_rx) = PeriodicScheduler::new(64);

    // Spawn the scheduler loop (runs in background)
    tokio::task::spawn_local(run_scheduler(backend, command_rx, 32));
    println!("   ✓ Scheduler created (capacity: 32 messages)\n");

    // Step 3: Add periodic messages with different intervals
    println!("3. Adding periodic messages...");

    // Message 1: Engine RPM (100ms interval)
    let msg1 = CanMessage::new_standard(0x100, &[0x00, 0x00, 0x0C, 0x80])?; // 3200 RPM
    let periodic1 = PeriodicMessage::new(msg1, Duration::from_millis(100))?;
    let id1 = scheduler.add(periodic1).await?;
    println!(
        "   ✓ Added message ID={}: CAN ID=0x100, Interval=100ms (Engine RPM)",
        id1
    );

    // Message 2: Vehicle Speed (50ms interval)
    let msg2 = CanMessage::new_standard(0x200, &[0x00, 0x50])?; // 80 km/h
    let periodic2 = PeriodicMessage::new(msg2, Duration::from_millis(50))?;
    let id2 = scheduler.add(periodic2).await?;
    println!(
        "   ✓ Added message ID={}: CAN ID=0x200, Interval=50ms (Vehicle Speed)",
        id2
    );

    // Message 3: Heartbeat (500ms interval)
    let msg3 = CanMessage::new_standard(0x700, &[0x05])?; // Node operational
    let periodic3 = PeriodicMessage::new(msg3, Duration::from_millis(500))?;
    let id3 = scheduler.add(periodic3).await?;
    println!(
        "   ✓ Added message ID={}: CAN ID=0x700, Interval=500ms (Heartbeat)",
        id3
    );

    println!();

    // Step 4: Let messages run for a while
    println!("4. Running periodic messages for 300ms...");
    tokio::time::sleep(Duration::from_millis(300)).await;
    println!("   ✓ Messages running\n");

    // Step 5: Check statistics
    println!("5. Checking statistics...");
    for (id, name) in [
        (id1, "Engine RPM"),
        (id2, "Vehicle Speed"),
        (id3, "Heartbeat"),
    ] {
        if let Ok(Some(stats)) = scheduler.get_stats(id).await {
            print!("   Message {} ({}): {} sends", id, name, stats.send_count());
            if let Some(avg) = stats.average_interval() {
                print!(", avg interval: {:?}", avg);
            }
            println!();
        }
    }
    println!();

    // Step 6: Dynamic updates
    println!("6. Performing dynamic updates...");

    // Update RPM data (simulate acceleration)
    scheduler
        .update_data(id1, vec![0x00, 0x00, 0x19, 0x00])
        .await?; // 6400 RPM
    println!("   ✓ Updated Engine RPM data to 6400 RPM");

    // Update speed interval (faster updates during acceleration)
    scheduler
        .update_interval(id2, Duration::from_millis(20))
        .await?;
    println!("   ✓ Updated Vehicle Speed interval to 20ms");

    // Temporarily disable heartbeat
    scheduler.set_enabled(id3, false).await?;
    println!("   ✓ Disabled Heartbeat message");

    println!();

    // Step 7: Run with updated configuration
    println!("7. Running with updated configuration for 200ms...");
    tokio::time::sleep(Duration::from_millis(200)).await;
    println!("   ✓ Updated messages running\n");

    // Step 8: Re-enable heartbeat
    println!("8. Re-enabling Heartbeat...");
    scheduler.set_enabled(id3, true).await?;
    println!("   ✓ Heartbeat re-enabled\n");

    // Step 9: List all active messages
    println!("9. Listing all active messages...");
    let ids = scheduler.list_ids().await;
    println!("   Active message IDs: {:?}", ids);
    println!();

    // Step 10: Final statistics
    println!("10. Final statistics...");
    for (id, name) in [
        (id1, "Engine RPM"),
        (id2, "Vehicle Speed"),
        (id3, "Heartbeat"),
    ] {
        if let Ok(Some(stats)) = scheduler.get_stats(id).await {
            println!("   Message {} ({}):", id, name);
            println!("      Total sends: {}", stats.send_count());
            if let Some(avg) = stats.average_interval() {
                println!("      Average interval: {:?}", avg);
            }
            if let Some(min) = stats.min_interval() {
                println!("      Min interval: {:?}", min);
            }
            if let Some(max) = stats.max_interval() {
                println!("      Max interval: {:?}", max);
            }
        }
    }
    println!();

    // Step 11: Remove a message
    println!("11. Removing Vehicle Speed message...");
    scheduler.remove(id2).await?;
    println!("   ✓ Message {} removed", id2);

    let remaining_ids = scheduler.list_ids().await;
    println!("   Remaining message IDs: {:?}", remaining_ids);
    println!();

    // Step 12: Shutdown
    println!("12. Shutting down scheduler...");
    scheduler.shutdown().await?;
    println!("   ✓ Scheduler shutdown complete");

    Ok(())
}
