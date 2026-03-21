//! List command implementation.
//!
//! Lists all available backends registered in the system.

use crate::error::CliResult;
use crate::output::{BackendListOutput, OutputFormatter};
use canlink_hal::BackendRegistry;

/// Execute the list command.
pub fn execute(formatter: &OutputFormatter) -> CliResult<()> {
    let registry = BackendRegistry::global();
    let backends = registry.list_backends();

    if formatter.is_json() {
        let output = BackendListOutput { backends };
        formatter.print(&output)?;
    } else if backends.is_empty() {
        formatter.print_message("No backends registered")?;
    } else {
        formatter.print_message("Available backends:")?;
        for backend in backends {
            println!("  - {}", backend);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use canlink_mock::MockBackendFactory;
    use std::sync::Arc;

    #[test]
    fn test_list_empty_registry() {
        let registry = BackendRegistry::new();
        let backends = registry.list_backends();
        assert_eq!(backends.len(), 0);
    }

    #[test]
    fn test_list_with_backends() {
        let registry = BackendRegistry::new();
        let factory = Arc::new(MockBackendFactory::new());
        registry.register(factory).unwrap();

        let backends = registry.list_backends();
        assert_eq!(backends.len(), 1);
        assert!(backends.contains(&"mock".to_string()));
    }
}
