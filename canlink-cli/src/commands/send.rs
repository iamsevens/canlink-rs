//! Send command implementation.
//!
//! Sends a CAN message through the specified backend.
//! Supports both single-shot and periodic sending modes.

use crate::error::{CliError, CliResult};
use crate::output::OutputFormatter;
use canlink_hal::{BackendConfig, BackendRegistry, CanBackend, CanMessage};
use std::time::Duration;

/// Execute the send command.
pub fn execute(
    backend_name: &str,
    channel: u32,
    id: u32,
    data: &[String],
    periodic_ms: Option<u64>,
    count: Option<u32>,
    formatter: &OutputFormatter,
) -> CliResult<()> {
    let registry = BackendRegistry::global();

    // Check if backend is registered
    if !registry.is_registered(backend_name) {
        return Err(CliError::BackendNotFound(backend_name.to_string()));
    }

    // Parse data bytes
    let data_bytes: Result<Vec<u8>, _> = data
        .iter()
        .map(|s| {
            let s = s.trim_start_matches("0x").trim_start_matches("0X");
            u8::from_str_radix(s, 16)
        })
        .collect();

    let data_bytes =
        data_bytes.map_err(|e| CliError::ParseError(format!("Invalid data byte: {}", e)))?;

    // Validate data length
    if data_bytes.len() > 8 {
        return Err(CliError::InvalidArgument(
            "CAN 2.0 messages cannot exceed 8 bytes. Use CAN-FD for larger messages.".to_string(),
        ));
    }

    // Create backend instance
    let config = BackendConfig::new(backend_name);
    let mut backend = registry.create(backend_name, &config)?;

    // Initialize backend
    backend.initialize(&config)?;

    // Open channel
    backend.open_channel(channel as u8)?;

    // Create message (determine if standard or extended ID)
    let message = if id <= 0x7FF {
        CanMessage::new_standard(id as u16, &data_bytes)?
    } else {
        CanMessage::new_extended(id, &data_bytes)?
    };

    // Check if periodic mode
    if let Some(interval_ms) = periodic_ms {
        execute_periodic(backend, message, interval_ms, count, channel, formatter)
    } else {
        execute_single(backend, message, channel, formatter)
    }
}

/// Execute single-shot send.
fn execute_single(
    mut backend: Box<dyn CanBackend>,
    message: CanMessage,
    channel: u32,
    formatter: &OutputFormatter,
) -> CliResult<()> {
    // Send message
    backend.send_message(&message)?;

    // Close channel and backend
    backend.close_channel(channel as u8)?;
    backend.close()?;

    // Report success
    let id_str = format!("0x{:X}", message.id().raw());
    let data_str = message
        .data()
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ");

    formatter.print_success(&format!(
        "Message sent: ID={}, Data=[{}], Channel={}",
        id_str, data_str, channel
    ))?;

    Ok(())
}

/// Execute periodic send using simple loop.
fn execute_periodic(
    mut backend: Box<dyn CanBackend>,
    message: CanMessage,
    interval_ms: u64,
    count: Option<u32>,
    channel: u32,
    formatter: &OutputFormatter,
) -> CliResult<()> {
    let id_str = format!("0x{:X}", message.id().raw());
    let data_str = message
        .data()
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ");

    let interval = Duration::from_millis(interval_ms);
    let send_count = count.unwrap_or(0); // 0 = infinite

    formatter.print_info(&format!(
        "Starting periodic send: ID={}, Data=[{}], Interval={}ms, Count={}, Channel={}",
        id_str,
        data_str,
        interval_ms,
        if send_count == 0 {
            "infinite".to_string()
        } else {
            send_count.to_string()
        },
        channel
    ))?;

    let mut sent_count = 0u32;
    let max_count = if send_count == 0 { 1000 } else { send_count }; // Safety limit

    loop {
        // Send message
        if let Err(e) = backend.send_message(&message) {
            formatter.print_error(&format!("Send failed after {} messages: {}", sent_count, e))?;
            break;
        }

        sent_count += 1;

        if sent_count >= max_count {
            if send_count == 0 {
                formatter.print_info("Reached 1000 messages safety limit")?;
            }
            break;
        }

        // Wait for next interval
        std::thread::sleep(interval);
    }

    // Close channel and backend
    backend.close_channel(channel as u8)?;
    backend.close()?;

    formatter.print_success(&format!(
        "Periodic send complete: {} messages sent",
        sent_count
    ))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_nonexistent_backend() {
        let _registry = BackendRegistry::new();
        let formatter = OutputFormatter::new(false);

        let result = execute("nonexistent", 0, 0x123, &[], None, None, &formatter);
        assert!(result.is_err());
    }
}
