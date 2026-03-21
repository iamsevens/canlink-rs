//! Queue overflow policy example (T053)
//!
//! This example demonstrates:
//! - Creating bounded queues with different overflow policies
//! - DropOldest: removes oldest messages when full
//! - DropNewest: rejects new messages when full
//! - Block: returns error when full (async version would block)
//! - Queue statistics and monitoring

use canlink_hal::message::CanMessage;
use canlink_hal::queue::{BoundedQueue, QueueOverflowPolicy};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== CANLink HAL - Queue Overflow Policy Example ===\n");

    // Step 1: Create a queue with DropOldest policy (default)
    println!("1. DropOldest Policy (default)");
    println!("   When queue is full, oldest messages are removed to make room.\n");

    let mut queue = BoundedQueue::with_policy(5, QueueOverflowPolicy::DropOldest);
    println!("   Created queue with capacity: {}", queue.capacity());

    // Fill the queue
    println!("   Filling queue with messages 1-5...");
    for i in 1..=5u16 {
        let msg = CanMessage::new_standard(i * 0x10, &[i as u8])?;
        queue.push(msg)?;
    }
    println!(
        "   Queue length: {}, is_full: {}",
        queue.len(),
        queue.is_full()
    );

    // Push more messages (will drop oldest)
    println!("   Pushing messages 6-8 (will drop oldest)...");
    for i in 6..=8u16 {
        let msg = CanMessage::new_standard(i * 0x10, &[i as u8])?;
        queue.push(msg)?;
    }

    // Show what's in the queue
    println!("   Queue contents after overflow:");
    for (i, msg) in queue.iter().enumerate() {
        println!("      [{}] ID: 0x{:X}", i, msg.id().raw());
    }

    let stats = queue.stats();
    println!("   Statistics:");
    println!("      Enqueued: {}", stats.enqueued);
    println!("      Dequeued: {}", stats.dequeued);
    println!("      Dropped: {}", stats.dropped);
    println!("      Overflow count: {}", stats.overflow_count);
    println!();

    // Step 2: Create a queue with DropNewest policy
    println!("2. DropNewest Policy");
    println!("   When queue is full, new messages are rejected.\n");

    let mut queue = BoundedQueue::with_policy(5, QueueOverflowPolicy::DropNewest);
    println!("   Created queue with capacity: {}", queue.capacity());

    // Fill the queue
    println!("   Filling queue with messages 1-5...");
    for i in 1..=5u16 {
        let msg = CanMessage::new_standard(i * 0x10, &[i as u8])?;
        queue.push(msg)?;
    }

    // Try to push more (will be rejected)
    println!("   Trying to push messages 6-8 (will be rejected)...");
    for i in 6..=8u16 {
        let msg = CanMessage::new_standard(i * 0x10, &[i as u8])?;
        match queue.push(msg) {
            Ok(_) => println!("      Message {} accepted", i),
            Err(e) => println!("      Message {} rejected: {}", i, e),
        }
    }

    // Show what's in the queue (should be original 1-5)
    println!("   Queue contents (unchanged):");
    for (i, msg) in queue.iter().enumerate() {
        println!("      [{}] ID: 0x{:X}", i, msg.id().raw());
    }

    let stats = queue.stats();
    println!("   Statistics:");
    println!("      Enqueued: {}", stats.enqueued);
    println!("      Dropped: {}", stats.dropped);
    println!();

    // Step 3: Create a queue with Block policy
    println!("3. Block Policy");
    println!("   When queue is full, returns QueueFull error (async would block).\n");

    let mut queue = BoundedQueue::with_policy(
        3,
        QueueOverflowPolicy::Block {
            timeout: Duration::from_millis(100),
        },
    );
    println!("   Created queue with capacity: {}", queue.capacity());

    // Fill the queue
    println!("   Filling queue with messages 1-3...");
    for i in 1..=3u16 {
        let msg = CanMessage::new_standard(i * 0x10, &[i as u8])?;
        queue.push(msg)?;
    }

    // Try to push more (will return error in sync mode)
    println!("   Trying to push message 4 (will return QueueFull error)...");
    let msg = CanMessage::new_standard(0x400, &[4])?;
    match queue.push(msg) {
        Ok(_) => println!("      Message accepted (unexpected)"),
        Err(e) => println!("      Error: {}", e),
    }
    println!();

    // Step 4: Demonstrate queue operations
    println!("4. Queue Operations Demo");

    let mut queue = BoundedQueue::new(10);
    println!("   Created default queue (DropOldest policy)");

    // Push some messages
    println!("   Pushing 5 messages...");
    for i in 1..=5u16 {
        let msg = CanMessage::new_standard(i * 0x10, &[i as u8])?;
        queue.push(msg)?;
    }
    println!("   Queue length: {}", queue.len());

    // Peek at front
    if let Some(front) = queue.peek() {
        println!("   Front message (peek): ID=0x{:X}", front.id().raw());
    }

    // Pop some messages
    println!("   Popping 2 messages...");
    for _ in 0..2 {
        if let Some(msg) = queue.pop() {
            println!("      Popped: ID=0x{:X}", msg.id().raw());
        }
    }
    println!("   Queue length after pop: {}", queue.len());

    // Clear the queue
    println!("   Clearing queue...");
    queue.clear();
    println!("   Queue length after clear: {}", queue.len());
    println!("   Queue is_empty: {}", queue.is_empty());
    println!();

    // Step 5: Demonstrate capacity adjustment
    println!("5. Capacity Adjustment");

    let mut queue = BoundedQueue::new(10);
    println!("   Initial capacity: {}", queue.capacity());

    // Fill with 8 messages
    println!("   Filling with 8 messages...");
    for i in 1..=8u16 {
        let msg = CanMessage::new_standard(i * 0x10, &[i as u8])?;
        queue.push(msg)?;
    }
    println!("   Queue length: {}", queue.len());

    // Reduce capacity (will drop oldest)
    println!("   Reducing capacity to 5 (will drop 3 oldest)...");
    queue.adjust_capacity(5);
    println!("   New capacity: {}", queue.capacity());
    println!("   Queue length: {}", queue.len());

    println!("   Remaining messages:");
    for (i, msg) in queue.iter().enumerate() {
        println!("      [{}] ID: 0x{:X}", i, msg.id().raw());
    }

    let stats = queue.stats();
    println!("   Dropped during resize: {}", stats.dropped);
    println!();

    // Step 6: High-throughput scenario
    println!("6. High-Throughput Scenario");

    let mut queue = BoundedQueue::with_policy(100, QueueOverflowPolicy::DropOldest);
    println!("   Queue capacity: 100, policy: DropOldest");

    // Simulate high message rate
    println!("   Simulating 1000 messages at high rate...");
    for i in 0..1000u32 {
        let msg = CanMessage::new_extended(i, &[(i % 256) as u8])?;
        queue.push(msg)?;
    }

    let stats = queue.stats();
    println!("   Final statistics:");
    println!("      Total enqueued: {}", stats.enqueued);
    println!("      Total dropped: {}", stats.dropped);
    println!("      Overflow events: {}", stats.overflow_count);
    println!("      Current queue length: {}", queue.len());
    println!();

    println!("=== Example completed successfully! ===");
    Ok(())
}
