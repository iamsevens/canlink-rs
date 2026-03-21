#!/bin/bash
# CLI Usage Script
#
# This script demonstrates how to use the canlink-cli tool for CAN bus operations.
# It covers common use cases and command patterns.

set -e  # Exit on error

echo "=== CANLink CLI Usage Examples ==="
echo ""

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print section headers
print_section() {
    echo ""
    echo -e "${BLUE}=== $1 ===${NC}"
    echo ""
}

# Function to print commands
print_command() {
    echo -e "${YELLOW}$ $1${NC}"
}

# Function to print success
print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

# Check if canlink-cli is available
if ! command -v canlink-cli &> /dev/null; then
    echo "Error: canlink-cli not found in PATH"
    echo "Please build it first: cargo build --release -p canlink-cli"
    exit 1
fi

print_section "1. List Available Backends"
print_command "canlink-cli list-backends"
canlink-cli list-backends
print_success "Listed available backends"

print_section "2. Get Backend Information"
print_command "canlink-cli backend-info mock"
canlink-cli backend-info mock
print_success "Retrieved backend information"

print_section "3. Send a Standard CAN Message"
print_command "canlink-cli send --backend mock --id 0x123 --data 01,02,03,04"
canlink-cli send --backend mock --id 0x123 --data 01,02,03,04
print_success "Sent standard CAN message"

print_section "4. Send an Extended CAN Message"
print_command "canlink-cli send --backend mock --id 0x12345678 --extended --data AA,BB,CC"
canlink-cli send --backend mock --id 0x12345678 --extended --data AA,BB,CC
print_success "Sent extended CAN message"

print_section "5. Send a CAN-FD Message"
print_command "canlink-cli send --backend mock --id 0x200 --canfd --data 01,02,03,04,05,06,07,08,09,0A,0B,0C"
canlink-cli send --backend mock --id 0x200 --canfd --data 01,02,03,04,05,06,07,08,09,0A,0B,0C
print_success "Sent CAN-FD message"

print_section "6. Send a Remote Frame"
print_command "canlink-cli send --backend mock --id 0x456 --remote --dlc 4"
canlink-cli send --backend mock --id 0x456 --remote --dlc 4
print_success "Sent remote frame"

print_section "7. Receive Messages (with timeout)"
print_command "canlink-cli receive --backend mock --timeout 1000"
echo "Note: This will timeout after 1 second if no messages are available"
canlink-cli receive --backend mock --timeout 1000 || true
print_success "Receive command completed"

print_section "8. Send Multiple Messages"
print_command "canlink-cli send --backend mock --id 0x100 --data 11,22,33"
canlink-cli send --backend mock --id 0x100 --data 11,22,33
print_command "canlink-cli send --backend mock --id 0x200 --data 44,55,66"
canlink-cli send --backend mock --id 0x200 --data 44,55,66
print_command "canlink-cli send --backend mock --id 0x300 --data 77,88,99"
canlink-cli send --backend mock --id 0x300 --data 77,88,99
print_success "Sent multiple messages"

print_section "9. Use Different Channels"
print_command "canlink-cli send --backend mock --channel 0 --id 0x111 --data 01"
canlink-cli send --backend mock --channel 0 --id 0x111 --data 01
print_command "canlink-cli send --backend mock --channel 1 --id 0x222 --data 02"
canlink-cli send --backend mock --channel 1 --id 0x222 --data 02
print_success "Used different channels"

print_section "10. Verbose Output"
print_command "canlink-cli send --backend mock --id 0x789 --data DE,AD,BE,EF --verbose"
canlink-cli send --backend mock --id 0x789 --data DE,AD,BE,EF --verbose
print_success "Sent with verbose output"

print_section "11. JSON Output Format"
print_command "canlink-cli backend-info mock --format json"
canlink-cli backend-info mock --format json
print_success "Retrieved info in JSON format"

print_section "12. Help Commands"
print_command "canlink-cli --help"
canlink-cli --help
echo ""
print_command "canlink-cli send --help"
canlink-cli send --help
print_success "Displayed help information"

print_section "Advanced Usage Examples"

echo "Example 1: OBD-II Request"
print_command "canlink-cli send --backend mock --id 0x7DF --data 02,01,0C"
echo "This sends an OBD-II request for engine RPM (PID 0x0C)"
canlink-cli send --backend mock --id 0x7DF --data 02,01,0C
print_success "Sent OBD-II request"

echo ""
echo "Example 2: J1939 Message"
print_command "canlink-cli send --backend mock --id 0x18FEF100 --extended --data 01,02,03,04,05,06,07,08"
echo "This sends a J1939 message with PGN 0xFEF1"
canlink-cli send --backend mock --id 0x18FEF100 --extended --data 01,02,03,04,05,06,07,08
print_success "Sent J1939 message"

echo ""
echo "Example 3: Batch Operations"
print_command "for i in {1..10}; do canlink-cli send --backend mock --id 0x\$i --data \$i,\$i,\$i; done"
echo "This sends 10 messages with sequential IDs"
for i in {1..10}; do
    canlink-cli send --backend mock --id 0x$i --data $i,$i,$i
done
print_success "Sent batch of messages"

print_section "Configuration File Example"
echo "You can create a configuration file at ~/.canlink/config.toml:"
echo ""
cat << 'EOF'
[default]
backend = "mock"
channel = 0
verbose = false

[backends.mock]
channel_count = 2
supports_canfd = true

[backends.socketcan]
interface = "can0"
EOF
echo ""
print_success "Configuration file example shown"

print_section "Environment Variables"
echo "You can also use environment variables:"
echo ""
echo "export CANLINK_BACKEND=mock"
echo "export CANLINK_CHANNEL=0"
echo "export CANLINK_VERBOSE=1"
echo ""
print_command "canlink-cli send --id 0x123 --data 01,02,03"
echo "(Uses environment variables for backend and channel)"
print_success "Environment variables explained"

print_section "Scripting Examples"

echo "Example: Monitor and log messages"
cat << 'EOF'
#!/bin/bash
# monitor.sh - Continuously monitor CAN bus
while true; do
    canlink-cli receive --backend mock --timeout 100 >> can_log.txt
done
EOF
echo ""

echo "Example: Send periodic messages"
cat << 'EOF'
#!/bin/bash
# heartbeat.sh - Send periodic heartbeat
while true; do
    canlink-cli send --backend mock --id 0x100 --data 01,02,03,04
    sleep 1
done
EOF
echo ""

print_success "Scripting examples shown"

print_section "Testing with Mock Backend"

echo "The mock backend is perfect for testing without hardware:"
echo ""
echo "1. Test your application logic"
print_command "canlink-cli send --backend mock --id 0x123 --data 01,02,03"
canlink-cli send --backend mock --id 0x123 --data 01,02,03

echo ""
echo "2. Verify message formats"
print_command "canlink-cli send --backend mock --id 0x456 --data AA,BB,CC,DD --verbose"
canlink-cli send --backend mock --id 0x456 --data AA,BB,CC,DD --verbose

echo ""
echo "3. Test error handling (mock backend always succeeds)"
print_command "canlink-cli send --backend mock --id 0xFFFFFFFF --extended --data FF"
canlink-cli send --backend mock --id 0xFFFFFFFF --extended --data FF || true

print_success "Mock backend testing examples completed"

print_section "Summary"
echo "This script demonstrated:"
echo "  ✓ Listing and querying backends"
echo "  ✓ Sending standard, extended, CAN-FD, and remote frames"
echo "  ✓ Receiving messages with timeout"
echo "  ✓ Using different channels"
echo "  ✓ Verbose and JSON output formats"
echo "  ✓ Advanced usage patterns (OBD-II, J1939)"
echo "  ✓ Batch operations and scripting"
echo "  ✓ Configuration and environment variables"
echo "  ✓ Testing with mock backend"
echo ""
echo -e "${GREEN}All examples completed successfully!${NC}"
echo ""
echo "For more information, see:"
echo "  - canlink-cli --help"
echo "  - Documentation: docs/cli-guide.md"
echo "  - Examples: examples/"
