//! Queue management module (FR-011, FR-017)
//!
//! This module provides bounded queue functionality with configurable
//! overflow policies for CAN message handling.
//!
//! # Overview
//!
//! The queue module provides a bounded message queue that prevents unbounded
//! memory growth when messages arrive faster than they can be processed.
//!
//! # Components
//!
//! - [`BoundedQueue`]: A fixed-capacity queue for CAN messages
//! - [`QueueOverflowPolicy`]: Strategy for handling queue overflow
//! - [`QueueStats`]: Statistics about queue operations
//! - [`QueueConfig`]: Configuration loaded from TOML files
//!
//! # Overflow Policies
//!
//! When the queue is full, one of these policies is applied:
//!
//! - [`QueueOverflowPolicy::DropOldest`]: Remove the oldest message (default)
//! - [`QueueOverflowPolicy::DropNewest`]: Reject the new message
//! - [`QueueOverflowPolicy::Block`]: Block until space is available (with timeout)
//!
//! # Example
//!
//! ```rust
//! use canlink_hal::queue::{BoundedQueue, QueueOverflowPolicy};
//! use canlink_hal::CanMessage;
//!
//! // Create a queue with capacity 100 and DropOldest policy
//! let mut queue = BoundedQueue::with_policy(100, QueueOverflowPolicy::DropOldest);
//!
//! // Push messages
//! let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
//! queue.push(msg).unwrap();
//!
//! // Pop messages
//! if let Some(msg) = queue.pop() {
//!     println!("Received: {:?}", msg);
//! }
//!
//! // Check statistics
//! let stats = queue.stats();
//! println!("Enqueued: {}, Dequeued: {}, Dropped: {}",
//!          stats.enqueued, stats.dequeued, stats.dropped);
//! ```
//!
//! # Thread Safety
//!
//! `BoundedQueue` is not thread-safe by itself. For multi-threaded access,
//! wrap it in appropriate synchronization primitives (e.g., `Mutex`, `RwLock`).

mod bounded;
mod config;
mod policy;

pub use bounded::{BoundedQueue, QueueStats};
pub use config::QueueConfig;
pub use policy::QueueOverflowPolicy;
