# CANLink CLI

A powerful command-line interface for interacting with CAN bus hardware through the CANLink Hardware Abstraction Layer.

## Features

- 🚀 **Backend Management**: List, query, and switch between different CAN hardware backends
- 📤 **Send Messages**: Send standard, extended, CAN-FD, and remote frames
- 📥 **Receive Messages**: Receive and display CAN messages with filtering
- ✅ **Configuration Validation**: Validate configuration files before use
- 🎨 **Multiple Output Formats**: Human-readable and JSON output
- 🔧 **Flexible Configuration**: Command-line arguments, environment variables, and config files

## Installation

### From Source

```bash
cargo install --path canlink-cli
```

### From Crates.io (when published)

```bash
cargo install canlink-cli
```

## Usage

### Quick Start

```bash
# List available backends
canlink list

# Query backend capabilities
canlink info mock

# Send a CAN message
canlink send mock 0 0x123 01 02 03 04

# Receive messages
canlink receive mock 0 --count 5
```

### List Available Backends

```bash
canlink list
```

Output:
```
Available backends:
  - mock
```

### Query Backend Information

```bash
canlink info <backend>
```

Example:
```bash
canlink info mock
```

Output:
```
Backend: mock
Version: 0.1.0
Channels: 2
CAN-FD Support: Yes
Max Bitrate: 8000000 bps
Supported Bitrates: [125000, 250000, 500000, 1000000]
Filter Count: 16
```

### Send a CAN Message

```bash
canlink send <backend> <channel> <id> [data...]
```

Example:
```bash
canlink send mock 0 0x123 01 02 03 04
```

Output:
```
✓ Message sent: ID=0x123, Data=[01 02 03 04], Channel=0
```

### Receive CAN Messages

```bash
canlink receive <backend> <channel> [--count <n>]
```

Example:
```bash
canlink receive mock 0 --count 5
```

### Validate Configuration File

```bash
canlink validate <config-file>
```

Example:
```bash
canlink validate canlink.toml
```

Output:
```
✓ Configuration file is valid: canlink.toml
  Backend: mock
```

## JSON Output

All commands support JSON output with the `--json` flag:

```bash
canlink --json info mock
```

Output:
```json
{
  "name": "mock",
  "version": "0.1.0",
  "channel_count": 2,
  "supports_canfd": true,
  "max_bitrate": 8000000,
  "supported_bitrates": [125000, 250000, 500000, 1000000],
  "filter_count": 16
}
```

## Configuration File Format

Create a `canlink.toml` file:

```toml
[backend]
backend_name = "mock"
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

## Examples

### Basic Usage

```bash
# List backends
canlink list

# Get backend info
canlink info mock

# Send a message
canlink send mock 0 0x123 AA BB CC

# Receive messages
canlink receive mock 0 --count 10
```

### Advanced Usage

```bash
# Send CAN-FD message (if supported)
canlink send mock 0 0x200 01 02 03 04 05 06 07 08 09 0A 0B 0C

# Send extended ID message
canlink send mock 0 0x18FEF100 01 02 03 04

# Receive with timeout
canlink receive mock 0 --count 1 --timeout 1000

# Filter by ID
canlink receive mock 0 --filter 0x123
```

### OBD-II Communication

```bash
# Request engine RPM (PID 0x0C)
canlink send mock 0 0x7DF 02 01 0C

# Receive response
canlink receive mock 0 --count 1 --filter 0x7E8
```

### Batch Operations

```bash
# Send multiple messages
for i in {1..10}; do
    canlink send mock 0 0x$i 0$i 0$i 0$i
done

# Monitor and log to file
canlink receive mock 0 --count 100 > can_log.txt
```

### JSON Mode

```bash
# Get backend info as JSON
canlink --json info mock | jq .

# List backends as JSON
canlink --json list
```

## Development

### Building from Source

```bash
git clone <your-public-repository-url>
cd canlink-rs
cargo build --release -p canlink-cli
```

### Running Tests

```bash
cargo test -p canlink-cli
```

### Running Examples

```bash
# See examples/cli_usage.sh for comprehensive examples
bash examples/cli_usage.sh
```

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

### Permission Denied

**Solution**: Ensure you have appropriate permissions to access CAN hardware

## Related Documentation

- [CANLink HAL Documentation](../canlink-hal/README.md)
- [Mock Backend Guide](../canlink-mock/README.md)
- [Examples](../examples/)
- [API Documentation](https://docs.rs/canlink-cli)

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

## License

MIT OR Apache-2.0
