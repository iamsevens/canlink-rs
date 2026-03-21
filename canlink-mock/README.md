# CANLink Mock Backend

A powerful mock implementation of the CAN hardware abstraction layer for testing without physical hardware.

## Features

- 🎯 **Message Recording**: Automatically records all sent messages for verification
- 📋 **Preset Messages**: Configure messages to be received in tests
- 💉 **Error Injection**: Simulate hardware errors and failures
- ✅ **Behavior Verification**: Validate application behavior with assertions
- ⚙️ **Configurable Capabilities**: Simulate different hardware configurations

## Installation

Add to your `Cargo.toml`:

```toml
[dev-dependencies]
canlink-mock = "0.1"
canlink-hal = "0.1"
```

## Quick Start

```rust
use canlink_hal::{BackendConfig, CanBackend, CanMessage, CanId};
use canlink_mock::MockBackend;

#[test]
fn test_can_communication() {
    // Create and initialize backend
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Send a message
    let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    backend.send_message(&msg).unwrap();

    // Verify it was recorded
    assert!(backend.verify_message_sent(CanId::Standard(0x123)));
    let recorded = backend.get_recorded_messages();
    assert_eq!(recorded.len(), 1);
}
```

## Core Features

### 1. Message Recording

All sent messages are automatically recorded:

```rust
use canlink_hal::{CanBackend, CanMessage, CanId};
use canlink_mock::MockBackend;

let mut backend = MockBackend::new();
// ... initialize and open channel ...

// Send messages
backend.send_message(&CanMessage::new_standard(0x100, &[1, 2]).unwrap()).unwrap();
backend.send_message(&CanMessage::new_standard(0x200, &[3, 4]).unwrap()).unwrap();
backend.send_message(&CanMessage::new_standard(0x100, &[5, 6]).unwrap()).unwrap();

// Verify total count
assert!(backend.verify_message_count(3));

// Verify specific ID was sent
assert!(backend.verify_message_sent(CanId::Standard(0x100)));

// Get all messages with specific ID
let messages = backend.get_messages_by_id(CanId::Standard(0x100));
assert_eq!(messages.len(), 2);
assert_eq!(messages[0].data(), &[1, 2]);
assert_eq!(messages[1].data(), &[5, 6]);

// Get all recorded messages
let all_messages = backend.get_recorded_messages();
assert_eq!(all_messages.len(), 3);

// Clear recorded messages
backend.clear_recorded_messages();
assert_eq!(backend.get_recorded_messages().len(), 0);
```

### 2. Preset Messages

Configure messages to be returned by `receive_message()`:

```rust
use canlink_hal::{CanBackend, CanMessage, CanId};
use canlink_mock::{MockBackend, MockConfig};

// Create preset messages
let preset = vec![
    CanMessage::new_standard(0x111, &[0x11, 0x22]).unwrap(),
    CanMessage::new_standard(0x222, &[0x33, 0x44]).unwrap(),
    CanMessage::new_standard(0x333, &[0x55, 0x66]).unwrap(),
];

// Create backend with preset messages
let config = MockConfig::with_preset_messages(preset);
let mut backend = MockBackend::with_config(config);
// ... initialize ...

// Receive preset messages in order
let msg1 = backend.receive_message().unwrap().unwrap();
assert_eq!(msg1.id(), CanId::Standard(0x111));

let msg2 = backend.receive_message().unwrap().unwrap();
assert_eq!(msg2.id(), CanId::Standard(0x222));

let msg3 = backend.receive_message().unwrap().unwrap();
assert_eq!(msg3.id(), CanId::Standard(0x333));

// No more messages
assert!(backend.receive_message().unwrap().is_none());
```

### 3. Error Injection

Simulate hardware errors for testing error handling:

```rust
use canlink_hal::{CanBackend, CanError, CanMessage};
use canlink_mock::MockBackend;

let mut backend = MockBackend::new();
// ... initialize ...

// Inject a send error
backend.error_injector_mut().inject_send_error(
    CanError::SendFailed {
        reason: "Bus-Off state".to_string(),
    }
);

// Next send will fail
let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
let result = backend.send_message(&msg);
assert!(result.is_err());

// Failed messages are not recorded
assert_eq!(backend.get_recorded_messages().len(), 0);
```

#### Advanced Error Injection

Control when errors occur with skip and count parameters:

```rust
// Fail the 3rd and 4th send attempts
backend.error_injector_mut().inject_send_error_with_config(
    CanError::SendFailed { reason: "Test".to_string() },
    2,  // inject 2 times
    2,  // skip first 2 calls
);

// First two sends succeed
backend.send_message(&msg1).unwrap(); // OK
backend.send_message(&msg2).unwrap(); // OK

// Next two fail
assert!(backend.send_message(&msg3).is_err());
assert!(backend.send_message(&msg4).is_err());

// Fifth succeeds (injection exhausted)
backend.send_message(&msg5).unwrap(); // OK
```

#### Error Types

You can inject errors for different operations:

```rust
// Send errors
backend.error_injector_mut().inject_send_error(error);

// Receive errors
backend.error_injector_mut().inject_receive_error(error);

// Initialization errors
backend.error_injector_mut().inject_init_error(error);

// Channel errors
backend.error_injector_mut().inject_open_channel_error(error);
backend.error_injector_mut().inject_close_channel_error(error);

// Clear all errors
backend.error_injector_mut().clear();
```

### 4. Configuration

Customize the mock backend behavior:

```rust
use canlink_mock::{MockBackend, MockConfig};

// CAN 2.0 only (no CAN-FD)
let config = MockConfig::can20_only();
let backend = MockBackend::with_config(config);

// Custom configuration
let mut config = MockConfig::new();
config.channel_count = 4;
config.supports_canfd = true;
config.max_bitrate = 2_000_000;
config.supported_bitrates = vec![125_000, 250_000, 500_000, 1_000_000];
config.filter_count = 32;

let backend = MockBackend::with_config(config);
```

## Testing Patterns

### Protocol Testing

Test request-response protocols like OBD-II:

```rust
// Setup preset responses
let responses = vec![
    CanMessage::new_standard(0x7E8, &[0x04, 0x41, 0x0C, 0x1A, 0xF8]).unwrap(),
];
let config = MockConfig::with_preset_messages(responses);
let mut backend = MockBackend::with_config(config);
// ... initialize ...

// Send request
let request = CanMessage::new_standard(0x7DF, &[0x02, 0x01, 0x0C]).unwrap();
backend.send_message(&request).unwrap();

// Receive response
let response = backend.receive_message().unwrap().unwrap();
assert_eq!(response.id(), CanId::Standard(0x7E8));

// Verify request was sent
assert!(backend.verify_message_sent(CanId::Standard(0x7DF)));
```

### Error Recovery Testing

Test retry logic:

```rust
// Fail first 3 attempts
backend.error_injector_mut().inject_send_error_with_config(
    CanError::SendFailed { reason: "Busy".to_string() },
    3,  // fail 3 times
    0,  // no skip
);

// Implement retry logic
let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
let mut attempts = 0;
let max_retries = 5;

while attempts < max_retries {
    match backend.send_message(&msg) {
        Ok(_) => break,
        Err(_) => attempts += 1,
    }
}

assert!(attempts < max_retries);
assert_eq!(backend.get_recorded_messages().len(), 1);
```

### Capability Adaptation Testing

Test that your application adapts to different hardware:

```rust
// Test with CAN-FD support
let mut backend_fd = MockBackend::new();
// ... initialize ...
let capability = backend_fd.get_capability().unwrap();
assert!(capability.supports_canfd);

// Test with CAN 2.0 only
let config_20 = MockConfig::can20_only();
let mut backend_20 = MockBackend::with_config(config_20);
// ... initialize ...
let capability = backend_20.get_capability().unwrap();
assert!(!capability.supports_canfd);
```

## Multi-Channel Testing

Test applications that use multiple CAN channels:

```rust
#[test]
fn test_multi_channel() {
    let mut config = MockConfig::default();
    config.channel_count = 4;
    let mut backend = MockBackend::with_config(config);
    // ... initialize ...

    // Open multiple channels
    backend.open_channel(0).unwrap();
    backend.open_channel(1).unwrap();
    backend.open_channel(2).unwrap();

    // Send messages on different channels
    backend.send_message(&CanMessage::new_standard(0x100, &[1]).unwrap()).unwrap();
    backend.send_message(&CanMessage::new_standard(0x200, &[2]).unwrap()).unwrap();
    backend.send_message(&CanMessage::new_standard(0x300, &[3]).unwrap()).unwrap();

    // Verify all messages recorded
    assert_eq!(backend.get_recorded_messages().len(), 3);
}
```

## Thread Safety

The mock backend is thread-safe and can be shared across threads:

```rust
use std::sync::{Arc, Mutex};
use std::thread;

let backend = Arc::new(Mutex::new(MockBackend::new()));

// Use from multiple threads
let backend_clone = Arc::clone(&backend);
let handle = thread::spawn(move || {
    let mut backend = backend_clone.lock().unwrap();
    backend.send_message(&msg).unwrap();
});

handle.join().unwrap();
```

## Performance Characteristics

The mock backend is designed for testing, not production use:

- **Message Recording**: O(1) insertion, capacity limited to 10,000 messages
- **FIFO Behavior**: When capacity exceeded, oldest messages are dropped
- **Thread-Safe**: Internal locking for concurrent access
- **No Timing Simulation**: No actual bus timing or arbitration

## Limitations

- Not suitable for performance testing (use real hardware)
- No actual bus timing or arbitration simulation
- Preset messages returned in FIFO order only
- Error injection is global (not per-channel)
- Recording capacity limited (configurable)

## Examples

See the `examples/` directory for complete examples:

- [`basic_usage.rs`](../examples/basic_usage.rs) - Basic mock backend usage
- [`mock_testing.rs`](../examples/mock_testing.rs) - Comprehensive mock testing demonstration
- [`automated_testing.rs`](../examples/automated_testing.rs) - Automated test suite using mock backend

## API Documentation

For detailed API documentation, see:
- [API Docs](https://docs.rs/canlink-mock)
- [Module Documentation](src/lib.rs)

### Key Types

- **`MockBackend`** - Main mock backend implementation
- **`MockConfig`** - Configuration for mock behavior
- **`MessageRecorder`** - Thread-safe message recording
- **`ErrorInjector`** - Error injection control

## Related Crates

- [`canlink-hal`](../canlink-hal) - Hardware abstraction layer
- [`canlink-cli`](../canlink-cli) - Command-line interface

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
