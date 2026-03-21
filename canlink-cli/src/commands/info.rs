//! Info command implementation.
//!
//! Queries and displays backend capabilities.

use crate::error::{CliError, CliResult};
use crate::output::{BackendInfoOutput, OutputFormatter};
use canlink_hal::{BackendConfig, BackendRegistry};

/// Execute the info command.
pub fn execute(backend_name: &str, formatter: &OutputFormatter) -> CliResult<()> {
    let registry = BackendRegistry::global();

    // Check if backend is registered
    if !registry.is_registered(backend_name) {
        return Err(CliError::BackendNotFound(backend_name.to_string()));
    }

    // Get backend info
    let info = registry
        .get_backend_info(backend_name)
        .map_err(|_| CliError::BackendNotFound(backend_name.to_string()))?;

    // Create backend instance to query capabilities
    let config = BackendConfig::new(backend_name);
    let backend = registry
        .create(backend_name, &config)
        .map_err(CliError::BackendError)?;

    let capability = backend.get_capability()?;

    // Format output
    let output = BackendInfoOutput {
        name: info.name.clone(),
        version: format!(
            "{}.{}.{}",
            info.version.major(),
            info.version.minor(),
            info.version.patch()
        ),
        channel_count: capability.channel_count as u32,
        supports_canfd: capability.supports_canfd,
        max_bitrate: capability.max_bitrate,
        supported_bitrates: capability.supported_bitrates.clone(),
        filter_count: capability.filter_count as u32,
    };

    if formatter.is_json() {
        formatter.print(&output)?;
    } else {
        println!("Backend: {}", output.name);
        println!("Version: {}", output.version);
        println!("Channels: {}", output.channel_count);
        println!(
            "CAN-FD Support: {}",
            if output.supports_canfd { "Yes" } else { "No" }
        );
        println!("Max Bitrate: {} bps", output.max_bitrate);
        println!("Supported Bitrates: {:?}", output.supported_bitrates);
        println!("Filter Count: {}", output.filter_count);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use canlink_mock::MockBackendFactory;
    use std::sync::Arc;

    #[test]
    fn test_info_nonexistent_backend() {
        let formatter = OutputFormatter::new(false);

        let result = execute("nonexistent", &formatter);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CliError::BackendNotFound(_)));
    }

    #[test]
    fn test_info_existing_backend() {
        let registry = BackendRegistry::global();
        let factory = Arc::new(MockBackendFactory::new());
        // Ignore error if already registered
        let _ = registry.register(factory);

        let formatter = OutputFormatter::new(true);
        let result = execute("mock", &formatter);
        assert!(result.is_ok());
    }
}
