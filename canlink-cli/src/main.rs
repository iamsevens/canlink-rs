//! # CANLink CLI
//!
//! Command-line interface for the CAN hardware abstraction layer.
//!
//! This tool provides commands for:
//! - Listing available backends
//! - Querying backend capabilities
//! - Sending and receiving CAN messages
//! - Periodic message sending (send --periodic)
//! - Validating configuration files

#![deny(missing_docs)]

mod commands;
mod error;
mod output;

use clap::{Parser, Subcommand};
use output::OutputFormatter;

/// Top-level CLI arguments.
#[derive(Parser)]
#[command(name = "canlink")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Top-level CLI command.
    #[command(subcommand)]
    command: Commands,

    /// Output in JSON format
    #[arg(short, long, global = true)]
    json: bool,
}

/// Supported top-level commands.
#[derive(Subcommand)]
enum Commands {
    /// List all available backends
    List,

    /// Query backend capabilities
    Info {
        /// Backend name
        backend: String,
    },

    /// Send a CAN message
    Send {
        /// Backend name
        backend: String,

        /// Channel number
        channel: u32,

        /// CAN ID (hex)
        #[arg(value_parser = parse_can_id)]
        id: u32,

        /// Data bytes (hex, space-separated)
        data: Vec<String>,

        /// Send periodically with specified interval in milliseconds
        #[arg(short, long, value_name = "MS")]
        periodic: Option<u64>,

        /// Number of messages to send in periodic mode (0 = infinite)
        #[arg(short = 'n', long, default_value = "0")]
        count: u32,
    },

    /// Receive CAN messages
    Receive {
        /// Backend name
        backend: String,

        /// Channel number
        channel: u32,

        /// Number of messages to receive (0 = continuous)
        #[arg(short, long, default_value = "1")]
        count: usize,
    },

    /// Validate configuration file
    Validate {
        /// Path to configuration file
        config: String,
    },
}

/// Parses a CAN ID from hex string (`0x123` or `123`).
fn parse_can_id(s: &str) -> Result<u32, String> {
    let s = s.trim_start_matches("0x").trim_start_matches("0X");
    u32::from_str_radix(s, 16).map_err(|e| format!("Invalid CAN ID: {}", e))
}

/// CLI entry point.
fn main() {
    // Register available backends
    use canlink_hal::BackendRegistry;
    use canlink_tscan::TSCanBackendFactory;
    use std::sync::Arc;

    let registry = BackendRegistry::global();

    // Register TSCan backend
    let tscan_factory = Arc::new(TSCanBackendFactory::new());
    if let Err(e) = registry.register(tscan_factory) {
        eprintln!("Warning: Failed to register tscan backend: {}", e);
    }

    let cli = Cli::parse();
    let formatter = OutputFormatter::new(cli.json);

    let result = match cli.command {
        Commands::List => commands::list::execute(&formatter),
        Commands::Info { backend } => commands::info::execute(&backend, &formatter),
        Commands::Send {
            backend,
            channel,
            id,
            data,
            periodic,
            count,
        } => commands::send::execute(
            &backend,
            channel,
            id,
            &data,
            periodic,
            if periodic.is_some() {
                Some(count)
            } else {
                None
            },
            &formatter,
        ),
        Commands::Receive {
            backend,
            channel,
            count,
        } => commands::receive::execute(&backend, channel, count, &formatter),
        Commands::Validate { config } => commands::validate::execute(&config, &formatter),
    };

    match result {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            let _ = formatter.print_error(&e.to_string());
            std::process::exit(e.exit_code());
        }
    }
}
