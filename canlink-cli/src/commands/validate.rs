//! Validate command implementation.
//!
//! Validates a configuration file.

use crate::error::{CliError, CliResult};
use crate::output::OutputFormatter;
use canlink_hal::CanlinkConfig;
use std::path::Path;

/// Execute the validate command.
pub fn execute(config_path: &str, formatter: &OutputFormatter) -> CliResult<()> {
    let path = Path::new(config_path);

    // Check if file exists
    if !path.exists() {
        return Err(CliError::ConfigError(format!(
            "Configuration file not found: {}",
            config_path
        )));
    }

    // Try to load the configuration
    match CanlinkConfig::from_file(config_path) {
        Ok(config) => {
            // Validate backend name is not empty
            if config.backend.backend_name.is_empty() {
                return Err(CliError::ConfigError(
                    "Backend name cannot be empty".to_string(),
                ));
            }

            // Additional validation could be added here
            // For example: check if backend exists, validate channel numbers, etc.

            if formatter.is_json() {
                let result = serde_json::json!({
                    "valid": true,
                    "backend": config.backend.backend_name,
                    "file": config_path
                });
                formatter.print(&result)?;
            } else {
                formatter
                    .print_success(&format!("Configuration file is valid: {}", config_path))?;
                println!("  Backend: {}", config.backend.backend_name);
            }

            Ok(())
        }
        Err(e) => Err(CliError::ConfigError(format!(
            "Invalid configuration file: {}",
            e
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_validate_nonexistent_file() {
        let formatter = OutputFormatter::new(false);
        let result = execute("/nonexistent/config.toml", &formatter);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CliError::ConfigError(_)));
    }

    #[test]
    fn test_validate_valid_config() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "[backend]").unwrap();
        writeln!(temp_file, "backend_name = \"mock\"").unwrap();
        temp_file.flush().unwrap();

        let formatter = OutputFormatter::new(false);
        let result = execute(temp_file.path().to_str().unwrap(), &formatter);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_invalid_toml() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "invalid toml content [[[").unwrap();
        temp_file.flush().unwrap();

        let formatter = OutputFormatter::new(false);
        let result = execute(temp_file.path().to_str().unwrap(), &formatter);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CliError::ConfigError(_)));
    }

    #[test]
    fn test_validate_empty_backend() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "[backend]").unwrap();
        writeln!(temp_file, "backend_name = \"\"").unwrap();
        temp_file.flush().unwrap();

        let formatter = OutputFormatter::new(false);
        let result = execute(temp_file.path().to_str().unwrap(), &formatter);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CliError::ConfigError(_)));
    }
}
