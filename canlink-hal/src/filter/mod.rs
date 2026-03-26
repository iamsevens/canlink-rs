//! Message filtering module (FR-005 to FR-009)
//!
//! This module provides message filtering functionality for CAN messages,
//! supporting both hardware and software filtering.
//!
//! # Overview
//!
//! The filtering system allows you to selectively receive CAN messages based on
//! their IDs. This is useful for reducing CPU load and focusing on relevant messages.
//!
//! # Filter Types
//!
//! - [`IdFilter`]: Match messages by exact ID or ID with mask
//! - [`RangeFilter`]: Match messages within an ID range
//! - [`FilterChain`]: Combine multiple filters with OR logic
//!
//! # Hardware vs Software Filtering
//!
//! Filters can be marked as hardware filters using the [`MessageFilter::is_hardware()`]
//! method. Hardware filters are applied by the CAN controller itself, reducing CPU load.
//! When hardware filter slots are exhausted, filters automatically fall back to software
//! filtering.
//!
//! # Example
//!
//! ```rust
//! use canlink_hal::filter::{FilterChain, IdFilter, RangeFilter, MessageFilter};
//! use canlink_hal::CanMessage;
//!
//! // Create a filter chain
//! let mut chain = FilterChain::new(8); // 8 hardware filter slots
//!
//! // Add filters (OR logic - message passes if ANY filter matches)
//! chain.add_filter(Box::new(IdFilter::new(0x123)));
//! chain.add_filter(Box::new(RangeFilter::new(0x200, 0x2FF)));
//!
//! // Test messages
//! let msg1 = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
//! let msg2 = CanMessage::new_standard(0x250, &[4, 5, 6]).unwrap();
//! let msg3 = CanMessage::new_standard(0x400, &[7, 8, 9]).unwrap();
//!
//! assert!(chain.matches(&msg1));  // Matches IdFilter
//! assert!(chain.matches(&msg2));  // Matches RangeFilter
//! assert!(!chain.matches(&msg3)); // No match
//! ```
//!
//! # Performance
//!
//! Software filtering is highly optimized with latency < 10 μs per message
//! (typically 3-20 ns). See SC-003 in the specification for details.

mod chain;
mod config;
mod id_filter;
mod range_filter;
mod traits;

pub use chain::FilterChain;
pub use config::FilterConfig;
pub use id_filter::IdFilter;
pub use range_filter::RangeFilter;
pub use traits::MessageFilter;
