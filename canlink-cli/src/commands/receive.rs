//! Receive command implementation.
//!
//! Receives CAN messages from the specified backend.

use crate::error::{CliError, CliResult};
use crate::output::{MessageOutput, OutputFormatter};
use canlink_hal::{BackendConfig, BackendRegistry, CanId};
use std::time::{Duration, Instant};

/// Execute the receive command.
pub fn execute(
    backend_name: &str,
    channel: u32,
    count: usize,
    formatter: &OutputFormatter,
) -> CliResult<()> {
    let registry = BackendRegistry::global();

    // Check if backend is registered
    if !registry.is_registered(backend_name) {
        return Err(CliError::BackendNotFound(backend_name.to_string()));
    }

    // Create backend instance
    let config = BackendConfig::new(backend_name);
    let mut backend = registry.create(backend_name, &config)?;

    // Initialize backend
    backend.initialize(&config)?;

    // Open channel
    backend.open_channel(channel as u8)?;

    // Receive messages
    let mut received = 0;
    let timeout = Duration::from_secs(5);
    let start = Instant::now();

    if !formatter.is_json() {
        if count == 0 {
            formatter.print_message("Receiving messages (Ctrl+C to stop)...")?;
        } else {
            formatter.print_message(&format!("Receiving {} message(s)...", count))?;
        }
    }

    loop {
        // Check timeout
        if start.elapsed() > timeout && received == 0 {
            backend.close_channel(channel as u8)?;
            backend.close()?;
            return Err(CliError::Timeout);
        }

        // Try to receive a message
        match backend.receive_message()? {
            Some(message) => {
                received += 1;

                // Format message
                let id_str = match message.id() {
                    CanId::Standard(id) => format!("0x{:03X}", id),
                    CanId::Extended(id) => format!("0x{:08X}", id),
                };

                let data_str = message
                    .data()
                    .iter()
                    .map(|b| format!("{:02X}", b))
                    .collect::<Vec<_>>()
                    .join(" ");

                let timestamp_str = message.timestamp().map(|ts| {
                    format!(
                        "{}.{:06}",
                        ts.as_micros() / 1_000_000,
                        ts.as_micros() % 1_000_000
                    )
                });

                let mut flags = Vec::new();
                if message.flags().contains(canlink_hal::MessageFlags::FD) {
                    flags.push("FD".to_string());
                }
                if message.flags().contains(canlink_hal::MessageFlags::BRS) {
                    flags.push("BRS".to_string());
                }
                if message.flags().contains(canlink_hal::MessageFlags::ESI) {
                    flags.push("ESI".to_string());
                }
                if message.flags().contains(canlink_hal::MessageFlags::RTR) {
                    flags.push("RTR".to_string());
                }

                let output = MessageOutput {
                    id: id_str,
                    data: data_str,
                    timestamp: timestamp_str,
                    flags,
                };

                if formatter.is_json() {
                    formatter.print(&output)?;
                } else {
                    println!("{}", output.format_human());
                }

                // Check if we've received enough messages
                if count > 0 && received >= count {
                    break;
                }
            }
            None => {
                // No message available, wait a bit
                std::thread::sleep(Duration::from_millis(10));

                // For non-continuous mode, check timeout
                if count > 0 && start.elapsed() > timeout {
                    break;
                }
            }
        }
    }

    // Close channel and backend
    backend.close_channel(channel as u8)?;
    backend.close()?;

    // Report result
    if received == 0 {
        return Err(CliError::NoMessages);
    }

    if !formatter.is_json() {
        formatter.print_success(&format!("Received {} message(s)", received))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_receive_nonexistent_backend() {
        let _registry = BackendRegistry::new();
        let formatter = OutputFormatter::new(false);

        let result = execute("nonexistent", 0, 1, &formatter);
        assert!(result.is_err());
    }
}
