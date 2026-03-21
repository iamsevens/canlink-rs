# CANLink CLI

Command-line interface for interacting with CAN hardware through the CANLink HAL.

## Features

- List available backends
- Query backend capabilities
- Send CAN messages (single-shot or periodic)
- Receive CAN messages
- Validate configuration files
- Human-readable and JSON output

## Installation

### From Source

```bash
cargo install --path canlink-cli
```

### From Crates.io (when published)

```bash
cargo install canlink-cli
```

## Requirements

Real hardware usage requires:

- Windows
- LibTSCAN runtime (TSMaster installation or a standalone LibTSCAN bundle)

## Quick Start

```bash
# List available backends
canlink list

# Query backend capabilities
canlink info tscan

# Send a CAN message
canlink send tscan 0 0x123 01 02 03 04

# Receive messages
canlink receive tscan 0 --count 5
```

## Commands

### List Available Backends

```bash
canlink list
```

### Query Backend Information

```bash
canlink info <backend>
```

Example:

```bash
canlink info tscan
```

### Send a CAN Message

```bash
canlink send <backend> <channel> <id> [data...]
```

Periodic mode:

```bash
canlink send tscan 0 0x123 01 02 03 04 --periodic 100 --count 10
```

### Receive CAN Messages

```bash
canlink receive <backend> <channel> [--count <n>]
```

### Validate Configuration File

```bash
canlink validate <config-file>
```

Example:

```bash
canlink validate canlink.toml
```

## JSON Output

All commands support JSON output with the `--json` flag:

```bash
canlink --json info tscan
```

## Configuration File Format

Create a `canlink.toml` file:

```toml
[backend]
backend_name = "tscan"
retry_count = 3
retry_interval_ms = 1000
```

## Exit Codes

- `0`: Success
- `2`: Backend not found
- `3`: Backend error
- `4`: Configuration error
- `5`: Invalid argument
- `6`: I/O error
- `7`: Parse error
- `8`: Timeout
- `9`: No messages received

## Troubleshooting

### Backend Not Found

```bash
Error: Backend 'socketcan' not found
```

**Solution**: Check available backends with `canlink list`

### Invalid Data Format

```bash
Error: Invalid hex byte: 'GG'
```

**Solution**: Use valid hex bytes (00-FF)

### Missing LibTSCAN Runtime

If `tscan` initialization fails, ensure the LibTSCAN runtime is available and matches the installed DLL/Lib bundle.

## Related Documentation

- [CANLink HAL Documentation](../canlink-hal/README.md)
- [TSMaster Backend Guide](../canlink-tscan/README.md)
- [Examples](../examples/)
- [API Documentation](https://docs.rs/canlink-cli)

## License

MIT OR Apache-2.0
