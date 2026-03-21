//! # CANLink CLI
//!
//! Command-line interface for the CAN hardware abstraction layer.
//!
//! This tool provides commands for:
//! - Listing available backends
//! - Querying backend capabilities
//! - Sending and receiving CAN messages
//! - Periodic message sending
//! - ISO-TP transport protocol support
//! - Validating configuration files
//! - Managing message filters
//! - Monitoring connection status

#![deny(missing_docs)]

mod commands;
mod error;
mod output;

use clap::{Parser, Subcommand};
use commands::filter::FilterType;
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

    /// Manage message filters
    #[command(subcommand)]
    Filter(FilterCommands),

    /// Monitor connection status
    #[command(subcommand)]
    Monitor(MonitorCommands),

    /// ISO-TP transport protocol commands
    #[command(subcommand)]
    Isotp(IsoTpCommands),
}

/// Filter subcommands
#[derive(Subcommand)]
enum FilterCommands {
    /// Add a new filter
    Add {
        /// Filter type: id, mask, or range
        #[arg(value_parser = parse_filter_type)]
        filter_type: FilterType,

        /// Filter parameters (depends on type)
        /// - id: `<id>`
        /// - mask: `<id> <mask>`
        /// - range: `<start> <end>`
        params: Vec<String>,

        /// Use extended (29-bit) CAN IDs
        #[arg(short, long)]
        extended: bool,
    },

    /// List all configured filters
    List,

    /// Remove a filter by index
    Remove {
        /// Filter index to remove
        index: usize,
    },

    /// Clear all filters
    Clear,
}

/// Monitor subcommands
#[derive(Subcommand)]
enum MonitorCommands {
    /// Display connection status
    Status,

    /// Attempt to reconnect to a backend
    Reconnect {
        /// Backend name to reconnect
        backend: String,
    },

    /// Configure monitor settings
    Config {
        /// Heartbeat interval in milliseconds
        #[arg(long)]
        heartbeat_ms: Option<u64>,

        /// Enable auto-reconnect
        #[arg(long)]
        auto_reconnect: bool,

        /// Maximum reconnect retries (requires --auto-reconnect)
        #[arg(long)]
        max_retries: Option<u32>,
    },
}

/// ISO-TP subcommands
#[derive(Subcommand)]
enum IsoTpCommands {
    /// Send an ISO-TP message
    Send {
        /// Backend name
        backend: String,

        /// Channel number
        #[arg(short, long, default_value = "0")]
        channel: u32,

        /// Transmit CAN ID (hex)
        #[arg(long, value_parser = parse_can_id)]
        tx_id: u32,

        /// Receive CAN ID (hex)
        #[arg(long, value_parser = parse_can_id)]
        rx_id: u32,

        /// Data bytes (hex, space-separated)
        data: Vec<String>,

        /// Timeout in milliseconds
        #[arg(short, long, default_value = "1000")]
        timeout: u64,
    },

    /// Receive an ISO-TP message
    Receive {
        /// Backend name
        backend: String,

        /// Channel number
        #[arg(short, long, default_value = "0")]
        channel: u32,

        /// Transmit CAN ID for Flow Control (hex)
        #[arg(long, value_parser = parse_can_id)]
        tx_id: u32,

        /// Receive CAN ID to listen for (hex)
        #[arg(long, value_parser = parse_can_id)]
        rx_id: u32,

        /// Timeout in milliseconds
        #[arg(short, long, default_value = "5000")]
        timeout: u64,
    },

    /// Send request and receive response (exchange)
    Exchange {
        /// Backend name
        backend: String,

        /// Channel number
        #[arg(short, long, default_value = "0")]
        channel: u32,

        /// Transmit CAN ID (hex)
        #[arg(long, value_parser = parse_can_id)]
        tx_id: u32,

        /// Receive CAN ID (hex)
        #[arg(long, value_parser = parse_can_id)]
        rx_id: u32,

        /// Request data bytes (hex, space-separated)
        data: Vec<String>,

        /// Timeout in milliseconds
        #[arg(short, long, default_value = "1000")]
        timeout: u64,
    },
}

/// Parses filter type name used by `filter add`.
fn parse_filter_type(s: &str) -> Result<FilterType, String> {
    s.parse()
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
    use canlink_mock::MockBackendFactory;
    use canlink_tscan::TSCanBackendFactory;
    use std::sync::Arc;

    let registry = BackendRegistry::global();

    // Register mock backend
    let mock_factory = Arc::new(MockBackendFactory::new());
    if let Err(e) = registry.register(mock_factory) {
        eprintln!("Warning: Failed to register mock backend: {}", e);
    }

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
        Commands::Filter(filter_cmd) => match filter_cmd {
            FilterCommands::Add {
                filter_type,
                params,
                extended,
            } => commands::filter::execute_add(filter_type, &params, extended, &formatter),
            FilterCommands::List => commands::filter::execute_list(&formatter),
            FilterCommands::Remove { index } => commands::filter::execute_remove(index, &formatter),
            FilterCommands::Clear => commands::filter::execute_clear(&formatter),
        },
        Commands::Monitor(monitor_cmd) => match monitor_cmd {
            MonitorCommands::Status => commands::monitor::execute_status(&formatter),
            MonitorCommands::Reconnect { backend } => {
                commands::monitor::execute_reconnect(&backend, &formatter)
            }
            MonitorCommands::Config {
                heartbeat_ms,
                auto_reconnect,
                max_retries,
            } => commands::monitor::configure_monitor(
                heartbeat_ms,
                auto_reconnect,
                max_retries,
                &formatter,
            ),
        },
        Commands::Isotp(isotp_cmd) => match isotp_cmd {
            IsoTpCommands::Send {
                backend,
                channel,
                tx_id,
                rx_id,
                data,
                timeout,
            } => commands::isotp::execute_send(
                &backend, channel, tx_id, rx_id, &data, timeout, &formatter,
            ),
            IsoTpCommands::Receive {
                backend,
                channel,
                tx_id,
                rx_id,
                timeout,
            } => commands::isotp::execute_receive(
                &backend, channel, tx_id, rx_id, timeout, &formatter,
            ),
            IsoTpCommands::Exchange {
                backend,
                channel,
                tx_id,
                rx_id,
                data,
                timeout,
            } => commands::isotp::execute_exchange(
                &backend, channel, tx_id, rx_id, &data, timeout, &formatter,
            ),
        },
    };

    match result {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            let _ = formatter.print_error(&e.to_string());
            std::process::exit(e.exit_code());
        }
    }
}
