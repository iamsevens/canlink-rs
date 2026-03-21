//! ISO-TP (ISO 15765-2) transport protocol module.
//!
//! This module provides functionality for transmitting data larger than a single CAN frame
//! using the ISO-TP segmentation and reassembly protocol.
//!
//! # Features
//!
//! - Single Frame (SF) for data ≤ 7 bytes (CAN 2.0) or ≤ 62 bytes (CAN-FD)
//! - Multi-frame transfer with First Frame (FF), Consecutive Frames (CF), and Flow Control (FC)
//! - Configurable block size and separation time (`STmin`)
//! - Support for Normal, Extended, and Mixed addressing modes
//! - Automatic CAN 2.0 / CAN-FD frame size detection
//!
//! # Example
//!
//! ```rust,ignore
//! use canlink_hal::isotp::{IsoTpChannel, IsoTpConfig};
//! use canlink_mock::MockBackend;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let backend = MockBackend::new();
//!
//!     let config = IsoTpConfig::builder()
//!         .tx_id(0x7E0)
//!         .rx_id(0x7E8)
//!         .build()?;
//!
//!     let mut channel = IsoTpChannel::new(backend, config)?;
//!
//!     // Send data (automatically segmented if needed)
//!     channel.send(&[0x10, 0x01]).await?;
//!
//!     // Receive response (automatically reassembled)
//!     let response = channel.receive().await?;
//!
//!     Ok(())
//! }
//! ```

mod channel;
mod config;
mod error;
mod frame;
mod state;

pub use channel::{IsoTpCallback, IsoTpChannel, NoOpCallback, TransferDirection};
pub use config::{AddressingMode, FrameSize, IsoTpConfig, IsoTpConfigBuilder};
pub use error::IsoTpError;
pub use frame::{FlowStatus, IsoTpFrame, StMin};
pub use state::{IsoTpState, RxState, TxState};
