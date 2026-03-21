//! ISO-TP command implementation.
//!
//! Provides ISO-TP (ISO 15765-2) transport protocol commands for sending
//! and receiving multi-frame CAN messages.
//!
//! Note: ISO-TP commands currently only work with the mock backend for testing.
//! Real hardware backends require async support to be added.

use crate::error::{CliError, CliResult};
use crate::output::OutputFormatter;
use canlink_hal::isotp::{IsoTpChannel, IsoTpConfig};
use canlink_hal::BackendRegistry;
use canlink_mock::{MockBackend, MockConfig};
use std::time::Duration;

/// Execute the isotp send command.
pub fn execute_send(
    backend_name: &str,
    _channel: u32,
    tx_id: u32,
    rx_id: u32,
    data: &[String],
    timeout_ms: u64,
    formatter: &OutputFormatter,
) -> CliResult<()> {
    // Currently only mock backend is supported for ISO-TP
    if backend_name != "mock" {
        return Err(CliError::InvalidArgument(
            "ISO-TP commands currently only support the 'mock' backend".to_string(),
        ));
    }

    let registry = BackendRegistry::global();
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

    if data_bytes.is_empty() {
        return Err(CliError::InvalidArgument(
            "Data cannot be empty".to_string(),
        ));
    }

    // Create ISO-TP config
    let isotp_config = IsoTpConfig::builder()
        .tx_id(tx_id)
        .rx_id(rx_id)
        .timeout(Duration::from_millis(timeout_ms))
        .build()
        .map_err(|e| CliError::OperationError(format!("Invalid ISO-TP config: {}", e)))?;

    let data_str = data_bytes
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ");

    formatter.print_info(&format!(
        "Sending ISO-TP message: TX_ID=0x{:X}, RX_ID=0x{:X}, Data=[{}], {} bytes",
        tx_id,
        rx_id,
        data_str,
        data_bytes.len()
    ))?;

    // Create mock backend
    let mock_config = MockConfig::default();
    let mut backend = MockBackend::with_config(mock_config);

    use canlink_hal::{BackendConfig, CanBackend};
    let config = BackendConfig::new("mock");
    backend.initialize(&config)?;
    backend.open_channel(0)?;

    // Run async send
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::OperationError(format!("Failed to create runtime: {}", e)))?;

    let local = tokio::task::LocalSet::new();

    let result = local.block_on(&rt, async {
        let mut isotp_channel = IsoTpChannel::new(backend, isotp_config).map_err(|e| {
            CliError::OperationError(format!("Failed to create ISO-TP channel: {}", e))
        })?;

        isotp_channel
            .send(&data_bytes)
            .await
            .map_err(|e| CliError::OperationError(format!("ISO-TP send failed: {}", e)))?;

        Ok::<(), CliError>(())
    });

    result?;

    formatter.print_success(&format!(
        "ISO-TP message sent successfully: {} bytes",
        data_bytes.len()
    ))?;

    Ok(())
}

/// Execute the isotp receive command.
pub fn execute_receive(
    backend_name: &str,
    _channel: u32,
    tx_id: u32,
    rx_id: u32,
    timeout_ms: u64,
    formatter: &OutputFormatter,
) -> CliResult<()> {
    // Currently only mock backend is supported for ISO-TP
    if backend_name != "mock" {
        return Err(CliError::InvalidArgument(
            "ISO-TP commands currently only support the 'mock' backend".to_string(),
        ));
    }

    let registry = BackendRegistry::global();
    if !registry.is_registered(backend_name) {
        return Err(CliError::BackendNotFound(backend_name.to_string()));
    }

    // Create ISO-TP config
    let isotp_config = IsoTpConfig::builder()
        .tx_id(tx_id)
        .rx_id(rx_id)
        .timeout(Duration::from_millis(timeout_ms))
        .build()
        .map_err(|e| CliError::OperationError(format!("Invalid ISO-TP config: {}", e)))?;

    formatter.print_info(&format!(
        "Waiting for ISO-TP message: TX_ID=0x{:X}, RX_ID=0x{:X}, Timeout={}ms",
        tx_id, rx_id, timeout_ms
    ))?;

    // Create mock backend
    let mock_config = MockConfig::default();
    let mut backend = MockBackend::with_config(mock_config);

    use canlink_hal::{BackendConfig, CanBackend};
    let config = BackendConfig::new("mock");
    backend.initialize(&config)?;
    backend.open_channel(0)?;

    // Run async receive
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::OperationError(format!("Failed to create runtime: {}", e)))?;

    let local = tokio::task::LocalSet::new();

    let result = local.block_on(&rt, async {
        let mut isotp_channel = IsoTpChannel::new(backend, isotp_config).map_err(|e| {
            CliError::OperationError(format!("Failed to create ISO-TP channel: {}", e))
        })?;

        let data = isotp_channel
            .receive()
            .await
            .map_err(|e| CliError::OperationError(format!("ISO-TP receive failed: {}", e)))?;

        Ok::<Vec<u8>, CliError>(data)
    });

    let data = result?;

    let data_str = data
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ");

    formatter.print_success(&format!(
        "ISO-TP message received: {} bytes, Data=[{}]",
        data.len(),
        data_str
    ))?;

    Ok(())
}

/// Execute the isotp exchange command (send request, receive response).
pub fn execute_exchange(
    backend_name: &str,
    _channel: u32,
    tx_id: u32,
    rx_id: u32,
    data: &[String],
    timeout_ms: u64,
    formatter: &OutputFormatter,
) -> CliResult<()> {
    // Currently only mock backend is supported for ISO-TP
    if backend_name != "mock" {
        return Err(CliError::InvalidArgument(
            "ISO-TP commands currently only support the 'mock' backend".to_string(),
        ));
    }

    let registry = BackendRegistry::global();
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

    if data_bytes.is_empty() {
        return Err(CliError::InvalidArgument(
            "Data cannot be empty".to_string(),
        ));
    }

    // Create ISO-TP config
    let isotp_config = IsoTpConfig::builder()
        .tx_id(tx_id)
        .rx_id(rx_id)
        .timeout(Duration::from_millis(timeout_ms))
        .build()
        .map_err(|e| CliError::OperationError(format!("Invalid ISO-TP config: {}", e)))?;

    let request_str = data_bytes
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ");

    formatter.print_info(&format!(
        "ISO-TP exchange: TX_ID=0x{:X}, RX_ID=0x{:X}, Request=[{}]",
        tx_id, rx_id, request_str
    ))?;

    // Create mock backend
    let mock_config = MockConfig::default();
    let mut backend = MockBackend::with_config(mock_config);

    use canlink_hal::{BackendConfig, CanBackend};
    let config = BackendConfig::new("mock");
    backend.initialize(&config)?;
    backend.open_channel(0)?;

    // Run async exchange
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::OperationError(format!("Failed to create runtime: {}", e)))?;

    let local = tokio::task::LocalSet::new();

    let result = local.block_on(&rt, async {
        let mut isotp_channel = IsoTpChannel::new(backend, isotp_config).map_err(|e| {
            CliError::OperationError(format!("Failed to create ISO-TP channel: {}", e))
        })?;

        // Send request
        isotp_channel
            .send(&data_bytes)
            .await
            .map_err(|e| CliError::OperationError(format!("ISO-TP send failed: {}", e)))?;

        // Receive response
        let response = isotp_channel
            .receive()
            .await
            .map_err(|e| CliError::OperationError(format!("ISO-TP receive failed: {}", e)))?;

        Ok::<Vec<u8>, CliError>(response)
    });

    let response = result?;

    let response_str = response
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ");

    formatter.print_success(&format!(
        "ISO-TP response: {} bytes, Data=[{}]",
        response.len(),
        response_str
    ))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use canlink_mock::MockBackendFactory;
    use std::sync::Arc;

    #[test]
    fn test_isotp_send_nonexistent_backend() {
        let _registry = BackendRegistry::new();
        let formatter = OutputFormatter::new(false);

        let result = execute_send("nonexistent", 0, 0x7E0, 0x7E8, &[], 1000, &formatter);
        assert!(result.is_err());
    }

    #[test]
    fn test_isotp_send_empty_data() {
        let registry = BackendRegistry::global();
        let factory = Arc::new(MockBackendFactory::new());
        let _ = registry.register(factory);

        let formatter = OutputFormatter::new(false);

        let result = execute_send("mock", 0, 0x7E0, 0x7E8, &[], 1000, &formatter);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CliError::InvalidArgument(_)));
    }

    #[test]
    fn test_isotp_unsupported_backend() {
        let formatter = OutputFormatter::new(false);

        let result = execute_send(
            "tscan",
            0,
            0x7E0,
            0x7E8,
            &["10".to_string(), "01".to_string()],
            1000,
            &formatter,
        );
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CliError::InvalidArgument(_)));
    }
}
