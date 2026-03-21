//! Integration tests for ISO-TP protocol.
//!
//! Tests cover:
//! - T020: IsoTpFrame encoding/decoding (all frame types)
//! - T021: SingleFrame send/receive
//! - T022: Multi-frame receive with auto FC response
//! - T023: Receive timeout (Scenario 2.3)
//! - T024: Sequence number wraparound
//! - T024a: Backend disconnect during receive (Scenario 2.4)
//! - T024b: Unexpected frame handling (Scenario 2.5)

mod common;
use canlink_hal::isotp::{
    AddressingMode, FlowStatus, FrameSize, IsoTpChannel, IsoTpConfig, IsoTpError, IsoTpFrame, StMin,
};
use canlink_hal::CanMessage;
use common::{create_backend_with_messages, create_initialized_backend, run_local};
use std::time::Duration;

// ============================================================================
// T020: IsoTpFrame encoding/decoding tests (all frame types)
// ============================================================================

#[test]
fn test_single_frame_encode_decode() {
    let data = vec![0x01, 0x02, 0x03, 0x04, 0x05];
    let frame = IsoTpFrame::SingleFrame {
        data_length: 5,
        data: data.clone(),
    };

    let encoded = frame.encode();
    assert_eq!(encoded[0], 0x05); // PCI: SF with length 5

    let decoded = IsoTpFrame::decode(&encoded).unwrap();
    match decoded {
        IsoTpFrame::SingleFrame {
            data_length,
            data: d,
        } => {
            assert_eq!(data_length, 5);
            assert_eq!(d, data);
        }
        _ => panic!("Expected SingleFrame"),
    }
}

#[test]
fn test_first_frame_encode_decode() {
    let data = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
    let frame = IsoTpFrame::FirstFrame {
        total_length: 100,
        data: data.clone(),
    };

    let encoded = frame.encode();
    assert_eq!(encoded[0], 0x10); // PCI: FF high nibble
    assert_eq!(encoded[1], 0x64); // Total length low byte (100)

    let decoded = IsoTpFrame::decode(&encoded).unwrap();
    match decoded {
        IsoTpFrame::FirstFrame {
            total_length,
            data: d,
        } => {
            assert_eq!(total_length, 100);
            assert_eq!(d, data);
        }
        _ => panic!("Expected FirstFrame"),
    }
}

#[test]
fn test_consecutive_frame_encode_decode() {
    let data = vec![0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D];
    let frame = IsoTpFrame::ConsecutiveFrame {
        sequence_number: 5,
        data: data.clone(),
    };

    let encoded = frame.encode();
    assert_eq!(encoded[0], 0x25); // PCI: CF with SN=5

    let decoded = IsoTpFrame::decode(&encoded).unwrap();
    match decoded {
        IsoTpFrame::ConsecutiveFrame {
            sequence_number,
            data: d,
        } => {
            assert_eq!(sequence_number, 5);
            assert_eq!(d, data);
        }
        _ => panic!("Expected ConsecutiveFrame"),
    }
}

#[test]
fn test_flow_control_encode_decode() {
    let frame = IsoTpFrame::FlowControl {
        flow_status: FlowStatus::ContinueToSend,
        block_size: 8,
        st_min: StMin::Milliseconds(20),
    };

    let encoded = frame.encode();
    assert_eq!(encoded[0], 0x30); // PCI: FC with CTS
    assert_eq!(encoded[1], 0x08); // BS=8
    assert_eq!(encoded[2], 0x14); // STmin=20ms

    let decoded = IsoTpFrame::decode(&encoded).unwrap();
    match decoded {
        IsoTpFrame::FlowControl {
            flow_status,
            block_size,
            st_min,
        } => {
            assert_eq!(flow_status, FlowStatus::ContinueToSend);
            assert_eq!(block_size, 8);
            assert_eq!(st_min, StMin::Milliseconds(20));
        }
        _ => panic!("Expected FlowControl"),
    }
}

#[test]
fn test_flow_control_wait() {
    let frame = IsoTpFrame::FlowControl {
        flow_status: FlowStatus::Wait,
        block_size: 0,
        st_min: StMin::Milliseconds(0),
    };

    let encoded = frame.encode();
    assert_eq!(encoded[0], 0x31); // PCI: FC with Wait

    let decoded = IsoTpFrame::decode(&encoded).unwrap();
    match decoded {
        IsoTpFrame::FlowControl { flow_status, .. } => {
            assert_eq!(flow_status, FlowStatus::Wait);
        }
        _ => panic!("Expected FlowControl"),
    }
}

#[test]
fn test_flow_control_overflow() {
    let frame = IsoTpFrame::FlowControl {
        flow_status: FlowStatus::Overflow,
        block_size: 0,
        st_min: StMin::Milliseconds(0),
    };

    let encoded = frame.encode();
    assert_eq!(encoded[0], 0x32); // PCI: FC with Overflow

    let decoded = IsoTpFrame::decode(&encoded).unwrap();
    match decoded {
        IsoTpFrame::FlowControl { flow_status, .. } => {
            assert_eq!(flow_status, FlowStatus::Overflow);
        }
        _ => panic!("Expected FlowControl"),
    }
}

#[test]
fn test_stmin_microseconds() {
    // Test microsecond encoding (0xF1-0xF9 = 100-900us)
    assert_eq!(StMin::from_byte(0xF1), StMin::Microseconds(100));
    assert_eq!(StMin::from_byte(0xF5), StMin::Microseconds(500));
    assert_eq!(StMin::from_byte(0xF9), StMin::Microseconds(900));

    assert_eq!(StMin::Microseconds(100).to_byte(), 0xF1);
    assert_eq!(StMin::Microseconds(500).to_byte(), 0xF5);
    assert_eq!(StMin::Microseconds(900).to_byte(), 0xF9);
}

#[test]
fn test_sequence_number_wraparound() {
    // Sequence numbers should wrap from 15 to 0
    for sn in 0..=15 {
        let frame = IsoTpFrame::ConsecutiveFrame {
            sequence_number: sn,
            data: vec![0x00],
        };
        let encoded = frame.encode();
        let decoded = IsoTpFrame::decode(&encoded).unwrap();
        match decoded {
            IsoTpFrame::ConsecutiveFrame {
                sequence_number, ..
            } => {
                assert_eq!(sequence_number, sn);
            }
            _ => panic!("Expected ConsecutiveFrame"),
        }
    }
}

#[test]
fn test_invalid_pci() {
    let data = [0x40, 0x00, 0x00]; // Invalid PCI type
    let result = IsoTpFrame::decode(&data);
    assert!(matches!(result, Err(IsoTpError::InvalidPci { pci: 0x40 })));
}

#[test]
fn test_empty_frame_data() {
    let result = IsoTpFrame::decode(&[]);
    assert!(matches!(result, Err(IsoTpError::InvalidFrame { .. })));
}

#[test]
fn test_sf_zero_length() {
    let data = [0x00]; // SF with length 0
    let result = IsoTpFrame::decode(&data);
    assert!(matches!(result, Err(IsoTpError::InvalidFrame { .. })));
}

// ============================================================================
// T021: SingleFrame send/receive tests
// ============================================================================

fn create_isotp_config() -> IsoTpConfig {
    IsoTpConfig::builder()
        .tx_id(0x7E0)
        .rx_id(0x7E8)
        .timeout(Duration::from_millis(100))
        .build()
        .unwrap()
}

#[test]
fn test_isotp_channel_creation() {
    let backend = create_initialized_backend();
    let config = create_isotp_config();

    let channel = IsoTpChannel::new(backend, config);
    assert!(channel.is_ok());

    let channel = channel.unwrap();
    assert!(channel.is_idle());
}

#[test]
fn test_isotp_config_validation() {
    // Invalid: standard ID > 0x7FF
    let result = IsoTpConfig::builder().tx_id(0x800).rx_id(0x7E8).build();
    assert!(matches!(result, Err(IsoTpError::InvalidConfig { .. })));

    // Valid: extended ID > 0x7FF
    let result = IsoTpConfig::builder()
        .tx_id(0x18DA00F1)
        .rx_id(0x18DAF100)
        .extended_ids(true)
        .build();
    assert!(result.is_ok());

    // Invalid: buffer size 0
    let result = IsoTpConfig::builder()
        .tx_id(0x7E0)
        .rx_id(0x7E8)
        .max_buffer_size(0)
        .build();
    assert!(matches!(result, Err(IsoTpError::InvalidConfig { .. })));

    // Invalid: buffer size > 4095
    let result = IsoTpConfig::builder()
        .tx_id(0x7E0)
        .rx_id(0x7E8)
        .max_buffer_size(5000)
        .build();
    assert!(matches!(result, Err(IsoTpError::InvalidConfig { .. })));
}

#[test]
fn test_isotp_id_boundaries() {
    let ok = IsoTpConfig::builder().tx_id(0x7FF).rx_id(0x7FF).build();
    assert!(ok.is_ok());

    let err = IsoTpConfig::builder().tx_id(0x800).rx_id(0x7FF).build();
    assert!(matches!(err, Err(IsoTpError::InvalidConfig { .. })));

    let ok = IsoTpConfig::builder()
        .tx_id(0x1FFF_FFFF)
        .rx_id(0x1FFF_FFFF)
        .extended_ids(true)
        .build();
    assert!(ok.is_ok());

    let err = IsoTpConfig::builder()
        .tx_id(0x2000_0000)
        .rx_id(0x1FFF_FFFF)
        .extended_ids(true)
        .build();
    assert!(matches!(err, Err(IsoTpError::InvalidConfig { .. })));
}

#[test]
fn test_send_single_frame() {
    run_local(async {
        let backend = create_initialized_backend();
        let config = create_isotp_config();
        let mut channel = IsoTpChannel::new(backend, config).unwrap();

        // Send small data (fits in SF)
        let data = vec![0x10, 0x01]; // Example: DiagSessionControl
        let result = channel.send(&data).await;
        assert!(result.is_ok(), "Send failed: {:?}", result.err());
    });
}

#[test]
fn test_send_empty_data_error() {
    run_local(async {
        let backend = create_initialized_backend();
        let config = create_isotp_config();
        let mut channel = IsoTpChannel::new(backend, config).unwrap();

        let result = channel.send(&[]).await;
        assert!(matches!(result, Err(IsoTpError::EmptyData)));
    });
}

#[test]
fn test_send_data_too_large_error() {
    run_local(async {
        let backend = create_initialized_backend();
        let config = IsoTpConfig::builder()
            .tx_id(0x7E0)
            .rx_id(0x7E8)
            .max_buffer_size(100)
            .timeout(Duration::from_millis(100))
            .build()
            .unwrap();
        let mut channel = IsoTpChannel::new(backend, config).unwrap();

        // Try to send data larger than max_buffer_size
        let data = vec![0x00; 200];
        let result = channel.send(&data).await;
        assert!(matches!(
            result,
            Err(IsoTpError::DataTooLarge {
                size: 200,
                max: 100
            })
        ));
    });
}

// ============================================================================
// T022: Multi-frame receive with auto FC response tests
// ============================================================================

#[test]
fn test_receive_single_frame() {
    run_local(async {
        // Preset a SF response
        let sf_data = vec![0x03, 0x50, 0x01, 0x00]; // SF: length=3, data=[0x50, 0x01, 0x00]
        let sf_msg = CanMessage::new_standard(0x7E8, &sf_data).unwrap();
        let backend = create_backend_with_messages(vec![sf_msg]);
        let config = create_isotp_config();

        let mut channel = IsoTpChannel::new(backend, config).unwrap();
        let result = channel.receive().await;

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data, vec![0x50, 0x01, 0x00]);
    });
}

#[test]
fn test_receive_multi_frame_small() {
    run_local(async {
        // Preset FF + CF sequence for 10 bytes of data
        // FF: total_length=10, first 6 bytes
        let ff_data = vec![0x10, 0x0A, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        let ff_msg = CanMessage::new_standard(0x7E8, &ff_data).unwrap();

        // CF1: SN=1, remaining 4 bytes
        let cf1_data = vec![0x21, 0x07, 0x08, 0x09, 0x0A, 0xCC, 0xCC, 0xCC];
        let cf1_msg = CanMessage::new_standard(0x7E8, &cf1_data).unwrap();

        let backend = create_backend_with_messages(vec![ff_msg, cf1_msg]);
        let config = IsoTpConfig::builder()
            .tx_id(0x7E0)
            .rx_id(0x7E8)
            .block_size(0) // No block size limit
            .st_min(StMin::Milliseconds(0))
            .timeout(Duration::from_millis(500))
            .build()
            .unwrap();

        let mut channel = IsoTpChannel::new(backend, config).unwrap();
        let result = channel.receive().await;

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(
            data,
            vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A]
        );
    });
}

// ============================================================================
// T023: Receive timeout test (Scenario 2.3)
// ============================================================================

#[test]
fn test_receive_timeout() {
    run_local(async {
        let backend = create_initialized_backend();
        let config = IsoTpConfig::builder()
            .tx_id(0x7E0)
            .rx_id(0x7E8)
            .timeout(Duration::from_millis(50)) // Short timeout
            .build()
            .unwrap();

        let mut channel = IsoTpChannel::new(backend, config).unwrap();

        // No messages preset, should timeout
        let result = channel.receive().await;
        assert!(matches!(result, Err(IsoTpError::RxTimeout { .. })));
    });
}

#[test]
fn test_receive_timeout_during_multiframe() {
    run_local(async {
        // Preset only FF, no CF follows
        let ff_data = vec![0x10, 0x14, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        let ff_msg = CanMessage::new_standard(0x7E8, &ff_data).unwrap();
        let backend = create_backend_with_messages(vec![ff_msg]);

        let config = IsoTpConfig::builder()
            .tx_id(0x7E0)
            .rx_id(0x7E8)
            .timeout(Duration::from_millis(50)) // Short timeout
            .build()
            .unwrap();

        let mut channel = IsoTpChannel::new(backend, config).unwrap();
        let result = channel.receive().await;

        // Should timeout waiting for CF
        assert!(matches!(result, Err(IsoTpError::RxTimeout { .. })));
        // Channel should be reset to idle
        assert!(channel.is_idle());
    });
}

// ============================================================================
// T024: Sequence number tests
// ============================================================================

#[test]
fn test_sequence_mismatch_error() {
    run_local(async {
        // Preset FF and CF with wrong sequence number
        let ff_data = vec![0x10, 0x14, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        let ff_msg = CanMessage::new_standard(0x7E8, &ff_data).unwrap();

        // CF with wrong sequence number (expected 1, got 2)
        let cf_data = vec![0x22, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D]; // SN=2 instead of 1
        let cf_msg = CanMessage::new_standard(0x7E8, &cf_data).unwrap();

        let backend = create_backend_with_messages(vec![ff_msg, cf_msg]);
        let config = IsoTpConfig::builder()
            .tx_id(0x7E0)
            .rx_id(0x7E8)
            .timeout(Duration::from_millis(200))
            .build()
            .unwrap();

        let mut channel = IsoTpChannel::new(backend, config).unwrap();
        let result = channel.receive().await;

        assert!(matches!(
            result,
            Err(IsoTpError::SequenceMismatch {
                expected: 1,
                actual: 2
            })
        ));
        assert!(channel.is_idle());
    });
}

// ============================================================================
// T024a: Backend disconnect during receive (Scenario 2.4)
// ============================================================================

// Note: Full backend disconnect testing requires MockBackend enhancements.
// For now, we test that errors are properly propagated.

#[test]
fn test_channel_abort() {
    let backend = create_initialized_backend();
    let config = create_isotp_config();
    let mut channel = IsoTpChannel::new(backend, config).unwrap();

    // Abort should work even when idle
    channel.abort();
    assert!(channel.is_idle());
}

// ============================================================================
// T024b: Unexpected frame handling (Scenario 2.5)
// ============================================================================

#[test]
fn test_unexpected_sf_during_receive() {
    run_local(async {
        // Preset FF to start multi-frame reception, then unexpected SF
        let ff_data = vec![0x10, 0x14, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        let ff_msg = CanMessage::new_standard(0x7E8, &ff_data).unwrap();

        // Unexpected SF instead of CF
        let sf_data = vec![0x03, 0xAA, 0xBB, 0xCC];
        let sf_msg = CanMessage::new_standard(0x7E8, &sf_data).unwrap();

        let backend = create_backend_with_messages(vec![ff_msg, sf_msg]);
        let config = IsoTpConfig::builder()
            .tx_id(0x7E0)
            .rx_id(0x7E8)
            .timeout(Duration::from_millis(200))
            .build()
            .unwrap();

        let mut channel = IsoTpChannel::new(backend, config).unwrap();
        let result = channel.receive().await;

        // Should return UnexpectedFrame error
        assert!(matches!(result, Err(IsoTpError::UnexpectedFrame { .. })));
        assert!(channel.is_idle());
    });
}

#[test]
fn test_unexpected_ff_during_receive() {
    run_local(async {
        // Preset FF to start multi-frame reception, then another FF
        let ff1_data = vec![0x10, 0x14, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        let ff1_msg = CanMessage::new_standard(0x7E8, &ff1_data).unwrap();

        // Another FF instead of CF (new transfer attempt)
        let ff2_data = vec![0x10, 0x0A, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
        let ff2_msg = CanMessage::new_standard(0x7E8, &ff2_data).unwrap();

        let backend = create_backend_with_messages(vec![ff1_msg, ff2_msg]);
        let config = IsoTpConfig::builder()
            .tx_id(0x7E0)
            .rx_id(0x7E8)
            .timeout(Duration::from_millis(200))
            .build()
            .unwrap();

        let mut channel = IsoTpChannel::new(backend, config).unwrap();
        let result = channel.receive().await;

        // Should return UnexpectedFrame error and send FC(Overflow)
        assert!(matches!(result, Err(IsoTpError::UnexpectedFrame { .. })));
        assert!(channel.is_idle());
    });
}

// ============================================================================
// Additional tests for frame size modes
// ============================================================================

#[test]
fn test_frame_size_classic8() {
    let config = IsoTpConfig::builder()
        .tx_id(0x7E0)
        .rx_id(0x7E8)
        .frame_size(FrameSize::Classic8)
        .build()
        .unwrap();

    // Classic8 should always use CAN 2.0 sizes
    assert_eq!(config.max_sf_data_length(true), 7); // Even with is_fd=true
    assert_eq!(config.max_sf_data_length(false), 7);
    assert_eq!(config.ff_data_length(true), 6);
    assert_eq!(config.cf_data_length(true), 7);
}

#[test]
fn test_frame_size_fd64() {
    let config = IsoTpConfig::builder()
        .tx_id(0x7E0)
        .rx_id(0x7E8)
        .frame_size(FrameSize::Fd64)
        .build()
        .unwrap();

    // FD64 should always use CAN-FD sizes
    assert_eq!(config.max_sf_data_length(false), 62); // Even with is_fd=false
    assert_eq!(config.max_sf_data_length(true), 62);
    assert_eq!(config.ff_data_length(false), 62);
    assert_eq!(config.cf_data_length(false), 63);
}

#[test]
fn test_frame_size_auto() {
    let config = IsoTpConfig::builder()
        .tx_id(0x7E0)
        .rx_id(0x7E8)
        .frame_size(FrameSize::Auto)
        .build()
        .unwrap();

    // Auto should depend on is_fd parameter
    assert_eq!(config.max_sf_data_length(false), 7);
    assert_eq!(config.max_sf_data_length(true), 62);
}

// ============================================================================
// T030: Multi-frame send tests (US3)
// ============================================================================

#[test]
fn test_send_multi_frame_needs_fc() {
    run_local(async {
        // Preset FC response for multi-frame send
        // FC: CTS, BS=0, STmin=0
        let fc_data = vec![0x30, 0x00, 0x00, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC];
        let fc_msg = CanMessage::new_standard(0x7E8, &fc_data).unwrap();
        let backend = create_backend_with_messages(vec![fc_msg]);

        let config = IsoTpConfig::builder()
            .tx_id(0x7E0)
            .rx_id(0x7E8)
            .timeout(Duration::from_millis(200))
            .build()
            .unwrap();

        let mut channel = IsoTpChannel::new(backend, config).unwrap();

        // Send data larger than SF (> 7 bytes for CAN 2.0)
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A];
        let result = channel.send(&data).await;
        assert!(
            result.is_ok(),
            "Multi-frame send failed: {:?}",
            result.err()
        );
        assert!(channel.is_idle());
    });
}

// ============================================================================
// T031: FC(Wait) handling tests (US3)
// ============================================================================

#[test]
fn test_send_fc_wait_then_cts() {
    run_local(async {
        // Preset FC(Wait) followed by FC(CTS)
        let fc_wait = vec![0x31, 0x00, 0x00, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC]; // FC Wait
        let fc_wait_msg = CanMessage::new_standard(0x7E8, &fc_wait).unwrap();
        let fc_cts = vec![0x30, 0x00, 0x00, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC]; // FC CTS
        let fc_cts_msg = CanMessage::new_standard(0x7E8, &fc_cts).unwrap();

        let backend = create_backend_with_messages(vec![fc_wait_msg, fc_cts_msg]);

        let config = IsoTpConfig::builder()
            .tx_id(0x7E0)
            .rx_id(0x7E8)
            .max_wait_count(5)
            .timeout(Duration::from_millis(500))
            .build()
            .unwrap();

        let mut channel = IsoTpChannel::new(backend, config).unwrap();

        // Send multi-frame data
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A];
        let result = channel.send(&data).await;
        assert!(
            result.is_ok(),
            "Send with FC(Wait) failed: {:?}",
            result.err()
        );
    });
}

// ============================================================================
// T032: Send timeout test (Scenario 3.3)
// ============================================================================

#[tokio::test(start_paused = true)]
async fn test_send_fc_timeout_exact_ms_paused() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let backend = create_initialized_backend();
            let config = IsoTpConfig::builder()
                .tx_id(0x7E0)
                .rx_id(0x7E8)
                .tx_timeout(Duration::from_millis(200))
                .rx_timeout(Duration::from_millis(200))
                .build()
                .unwrap();

            let mut channel = IsoTpChannel::new(backend, config).unwrap();

            let (tx, rx) = tokio::sync::oneshot::channel();
            tokio::task::spawn_local(async move {
                let data = vec![0x01; 10];
                let result = channel.send(&data).await;
                let _ = tx.send(result);
            });

            tokio::time::advance(Duration::from_millis(200)).await;
            tokio::task::yield_now().await;

            let result = rx.await.unwrap();
            assert!(matches!(
                result,
                Err(IsoTpError::FcTimeout { timeout_ms: 200 })
            ));
        })
        .await;
}

#[tokio::test(start_paused = true)]
async fn test_receive_timeout_exact_ms_paused() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let backend = create_initialized_backend();
            let config = IsoTpConfig::builder()
                .tx_id(0x7E0)
                .rx_id(0x7E8)
                .rx_timeout(Duration::from_millis(150))
                .tx_timeout(Duration::from_millis(150))
                .build()
                .unwrap();

            let mut channel = IsoTpChannel::new(backend, config).unwrap();

            let (tx, rx) = tokio::sync::oneshot::channel();
            tokio::task::spawn_local(async move {
                let result = channel.receive().await;
                let _ = tx.send(result);
            });

            tokio::time::advance(Duration::from_millis(150)).await;
            tokio::task::yield_now().await;

            let result = rx.await.unwrap();
            assert!(matches!(
                result,
                Err(IsoTpError::RxTimeout { timeout_ms: 150 })
            ));
        })
        .await;
}

#[test]
fn test_send_fc_timeout() {
    run_local(async {
        // No FC response preset - should timeout
        let backend = create_initialized_backend();

        let config = IsoTpConfig::builder()
            .tx_id(0x7E0)
            .rx_id(0x7E8)
            .timeout(Duration::from_millis(50)) // Short timeout
            .build()
            .unwrap();

        let mut channel = IsoTpChannel::new(backend, config).unwrap();

        // Send multi-frame data (needs FC)
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A];
        let result = channel.send(&data).await;

        assert!(matches!(result, Err(IsoTpError::FcTimeout { .. })));
        assert!(channel.is_idle());
    });
}

// ============================================================================
// T033: Round-trip test (send and receive)
// ============================================================================

#[test]
fn test_single_frame_roundtrip() {
    run_local(async {
        // Preset a response for our request
        let response_data = vec![0x03, 0x50, 0x01, 0x00, 0xCC, 0xCC, 0xCC, 0xCC]; // SF response
        let response_msg = CanMessage::new_standard(0x7E8, &response_data).unwrap();
        let backend = create_backend_with_messages(vec![response_msg]);

        let config = create_isotp_config();
        let mut channel = IsoTpChannel::new(backend, config).unwrap();

        // Send request
        let request = vec![0x10, 0x01]; // DiagSessionControl
        let send_result = channel.send(&request).await;
        assert!(send_result.is_ok());

        // Receive response
        let recv_result = channel.receive().await;
        assert!(recv_result.is_ok());
        assert_eq!(recv_result.unwrap(), vec![0x50, 0x01, 0x00]);
    });
}

// ============================================================================
// T033a: TooManyWaits test (Scenario 3.4)
// ============================================================================

#[test]
fn test_send_too_many_waits() {
    run_local(async {
        // Preset multiple FC(Wait) responses exceeding max_wait_count
        let fc_wait = vec![0x31, 0x00, 0x00, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC];
        let fc_wait_msg1 = CanMessage::new_standard(0x7E8, &fc_wait).unwrap();
        let fc_wait_msg2 = CanMessage::new_standard(0x7E8, &fc_wait).unwrap();
        let fc_wait_msg3 = CanMessage::new_standard(0x7E8, &fc_wait).unwrap();

        let backend = create_backend_with_messages(vec![fc_wait_msg1, fc_wait_msg2, fc_wait_msg3]);

        let config = IsoTpConfig::builder()
            .tx_id(0x7E0)
            .rx_id(0x7E8)
            .max_wait_count(2) // Only allow 2 waits
            .timeout(Duration::from_millis(500))
            .build()
            .unwrap();

        let mut channel = IsoTpChannel::new(backend, config).unwrap();

        // Send multi-frame data
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A];
        let result = channel.send(&data).await;

        assert!(matches!(
            result,
            Err(IsoTpError::TooManyWaits { count: 3, max: 2 })
        ));
        assert!(channel.is_idle());
    });
}

// ============================================================================
// T033b: Non-FC frame ignored during send (Scenario 3.5)
// ============================================================================

#[test]
fn test_send_ignores_non_fc_frames() {
    run_local(async {
        // Preset a SF (should be ignored) followed by FC(CTS)
        let sf_data = vec![0x03, 0xAA, 0xBB, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC]; // SF - should be ignored
        let sf_msg = CanMessage::new_standard(0x7E8, &sf_data).unwrap();
        let fc_cts = vec![0x30, 0x00, 0x00, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC]; // FC CTS
        let fc_cts_msg = CanMessage::new_standard(0x7E8, &fc_cts).unwrap();

        let backend = create_backend_with_messages(vec![sf_msg, fc_cts_msg]);

        let config = IsoTpConfig::builder()
            .tx_id(0x7E0)
            .rx_id(0x7E8)
            .timeout(Duration::from_millis(500))
            .build()
            .unwrap();

        let mut channel = IsoTpChannel::new(backend, config).unwrap();

        // Send multi-frame data
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A];
        let result = channel.send(&data).await;

        // Should succeed - SF was ignored, FC(CTS) was processed
        assert!(result.is_ok(), "Send failed: {:?}", result.err());
    });
}

// ============================================================================
// T033c: Abort and state cleanup test (Scenario 3.6)
// ============================================================================

#[test]
fn test_abort_clears_state() {
    let backend = create_initialized_backend();
    let config = create_isotp_config();
    let mut channel = IsoTpChannel::new(backend, config).unwrap();

    // Channel should start idle
    assert!(channel.is_idle());

    // Abort on idle should be safe
    channel.abort();
    assert!(channel.is_idle());
}

#[test]
fn test_send_fc_overflow() {
    run_local(async {
        // Preset FC(Overflow) response
        let fc_overflow = vec![0x32, 0x00, 0x00, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC];
        let fc_overflow_msg = CanMessage::new_standard(0x7E8, &fc_overflow).unwrap();

        let backend = create_backend_with_messages(vec![fc_overflow_msg]);

        let config = IsoTpConfig::builder()
            .tx_id(0x7E0)
            .rx_id(0x7E8)
            .timeout(Duration::from_millis(200))
            .build()
            .unwrap();

        let mut channel = IsoTpChannel::new(backend, config).unwrap();

        // Send multi-frame data
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A];
        let result = channel.send(&data).await;

        assert!(matches!(result, Err(IsoTpError::RemoteOverflow)));
        assert!(channel.is_idle());
    });
}

// ============================================================================
// T037: Callback tests
// ============================================================================

use canlink_hal::isotp::{IsoTpCallback, TransferDirection};
use std::sync::{Arc, Mutex};

struct TestCallback {
    events: Arc<Mutex<Vec<String>>>,
}

impl TestCallback {
    fn new() -> (Self, Arc<Mutex<Vec<String>>>) {
        let events = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                events: events.clone(),
            },
            events,
        )
    }
}

impl IsoTpCallback for TestCallback {
    fn on_transfer_start(&mut self, direction: TransferDirection, total_length: usize) {
        self.events
            .lock()
            .unwrap()
            .push(format!("start:{:?}:{}", direction, total_length));
    }

    fn on_transfer_complete(&mut self, direction: TransferDirection, total_bytes: usize) {
        self.events
            .lock()
            .unwrap()
            .push(format!("complete:{:?}:{}", direction, total_bytes));
    }

    fn on_transfer_error(&mut self, direction: TransferDirection, error: &IsoTpError) {
        self.events
            .lock()
            .unwrap()
            .push(format!("error:{:?}:{:?}", direction, error));
    }
}

#[test]
fn test_callback_on_send() {
    run_local(async {
        let backend = create_initialized_backend();
        let config = create_isotp_config();
        let mut channel = IsoTpChannel::new(backend, config).unwrap();

        let (callback, events) = TestCallback::new();
        channel.set_callback(callback);

        // Send single frame
        let data = vec![0x10, 0x01];
        let _ = channel.send(&data).await;

        let events = events.lock().unwrap();
        assert!(events.iter().any(|e| e.contains("start:Send:2")));
        assert!(events.iter().any(|e| e.contains("complete:Send:2")));
    });
}

#[test]
fn test_callback_on_receive() {
    run_local(async {
        let sf_data = vec![0x03, 0x50, 0x01, 0x00];
        let sf_msg = CanMessage::new_standard(0x7E8, &sf_data).unwrap();
        let backend = create_backend_with_messages(vec![sf_msg]);
        let config = create_isotp_config();
        let mut channel = IsoTpChannel::new(backend, config).unwrap();

        let (callback, events) = TestCallback::new();
        channel.set_callback(callback);

        let _ = channel.receive().await;

        let events = events.lock().unwrap();
        assert!(events.iter().any(|e| e.contains("start:Receive:3")));
        assert!(events.iter().any(|e| e.contains("complete:Receive:3")));
    });
}

// ============================================================================
// T041: Extended/Mixed addressing mode tests (FR-015)
// ============================================================================

#[test]
fn test_extended_addressing_config() {
    // Test Extended addressing mode configuration
    let config = IsoTpConfig::builder()
        .tx_id(0x7E0)
        .rx_id(0x7E8)
        .addressing_mode(AddressingMode::Extended {
            target_address: 0x55,
        })
        .build()
        .unwrap();

    assert!(matches!(
        config.addressing_mode,
        AddressingMode::Extended {
            target_address: 0x55
        }
    ));

    // Extended addressing reduces data capacity by 1 byte
    assert_eq!(config.max_sf_data_length(false), 6); // 7 - 1 = 6
    assert_eq!(config.ff_data_length(false), 5); // 6 - 1 = 5
    assert_eq!(config.cf_data_length(false), 6); // 7 - 1 = 6
}

#[test]
fn test_mixed_addressing_config() {
    // Test Mixed addressing mode configuration
    let config = IsoTpConfig::builder()
        .tx_id(0x7E0)
        .rx_id(0x7E8)
        .addressing_mode(AddressingMode::Mixed {
            address_extension: 0xF1,
        })
        .build()
        .unwrap();

    assert!(matches!(
        config.addressing_mode,
        AddressingMode::Mixed {
            address_extension: 0xF1
        }
    ));

    // Mixed addressing also reduces data capacity by 1 byte
    assert_eq!(config.max_sf_data_length(false), 6);
    assert_eq!(config.ff_data_length(false), 5);
    assert_eq!(config.cf_data_length(false), 6);
}

#[test]
fn test_extended_addressing_send_single_frame() {
    run_local(async {
        let backend = create_initialized_backend();
        let config = IsoTpConfig::builder()
            .tx_id(0x7E0)
            .rx_id(0x7E8)
            .addressing_mode(AddressingMode::Extended {
                target_address: 0x55,
            })
            .timeout(Duration::from_millis(100))
            .build()
            .unwrap();

        let mut channel = IsoTpChannel::new(backend, config).unwrap();

        // Send a small message (should fit in SF with extended addressing)
        let data = vec![0x10, 0x01]; // 2 bytes
        let result = channel.send(&data).await;
        assert!(result.is_ok());

        // The sent frame should have address byte prepended:
        // [0x55, 0x02, 0x10, 0x01, padding...]
        // Where 0x55 is target address, 0x02 is SF PCI with length 2
    });
}

#[test]
fn test_extended_addressing_receive_single_frame() {
    run_local(async {
        // Create a response with extended addressing format:
        // [address_byte, PCI, data...]
        // For SF: [0x55, 0x03, 0x50, 0x01, 0x00]
        let sf_data = vec![0x55, 0x03, 0x50, 0x01, 0x00, 0xCC, 0xCC, 0xCC];
        let sf_msg = CanMessage::new_standard(0x7E8, &sf_data).unwrap();
        let backend = create_backend_with_messages(vec![sf_msg]);

        let config = IsoTpConfig::builder()
            .tx_id(0x7E0)
            .rx_id(0x7E8)
            .addressing_mode(AddressingMode::Extended {
                target_address: 0x55,
            })
            .timeout(Duration::from_millis(100))
            .build()
            .unwrap();

        let mut channel = IsoTpChannel::new(backend, config).unwrap();

        let result = channel.receive().await;
        assert!(result.is_ok());

        let received = result.unwrap();
        assert_eq!(received, vec![0x50, 0x01, 0x00]);
    });
}

#[test]
fn test_mixed_addressing_send_single_frame() {
    run_local(async {
        let backend = create_initialized_backend();
        let config = IsoTpConfig::builder()
            .tx_id(0x7E0)
            .rx_id(0x7E8)
            .addressing_mode(AddressingMode::Mixed {
                address_extension: 0xF1,
            })
            .timeout(Duration::from_millis(100))
            .build()
            .unwrap();

        let mut channel = IsoTpChannel::new(backend, config).unwrap();

        // Send a small message
        let data = vec![0x3E, 0x00]; // TesterPresent
        let result = channel.send(&data).await;
        assert!(result.is_ok());

        // The sent frame should have address extension prepended:
        // [0xF1, 0x02, 0x3E, 0x00, padding...]
    });
}

#[test]
fn test_mixed_addressing_receive_single_frame() {
    run_local(async {
        // Create a response with mixed addressing format:
        // [address_extension, PCI, data...]
        let sf_data = vec![0xF1, 0x02, 0x7E, 0x00, 0xCC, 0xCC, 0xCC, 0xCC];
        let sf_msg = CanMessage::new_standard(0x7E8, &sf_data).unwrap();
        let backend = create_backend_with_messages(vec![sf_msg]);

        let config = IsoTpConfig::builder()
            .tx_id(0x7E0)
            .rx_id(0x7E8)
            .addressing_mode(AddressingMode::Mixed {
                address_extension: 0xF1,
            })
            .timeout(Duration::from_millis(100))
            .build()
            .unwrap();

        let mut channel = IsoTpChannel::new(backend, config).unwrap();

        let result = channel.receive().await;
        assert!(result.is_ok());

        let received = result.unwrap();
        assert_eq!(received, vec![0x7E, 0x00]);
    });
}

#[test]
fn test_mixed_addressing_address_mismatch() {
    run_local(async {
        // Create a response with wrong address extension
        let sf_data = vec![0xAA, 0x02, 0x7E, 0x00, 0xCC, 0xCC, 0xCC, 0xCC]; // 0xAA != 0xF1
        let sf_msg = CanMessage::new_standard(0x7E8, &sf_data).unwrap();
        let backend = create_backend_with_messages(vec![sf_msg]);

        let config = IsoTpConfig::builder()
            .tx_id(0x7E0)
            .rx_id(0x7E8)
            .addressing_mode(AddressingMode::Mixed {
                address_extension: 0xF1,
            })
            .timeout(Duration::from_millis(100))
            .build()
            .unwrap();

        let mut channel = IsoTpChannel::new(backend, config).unwrap();

        // Should fail due to address mismatch
        let result = channel.receive().await;
        assert!(result.is_err());
        assert!(matches!(result, Err(IsoTpError::InvalidFrame { .. })));
    });
}

#[test]
fn test_extended_addressing_multi_frame_receive() {
    run_local(async {
        // Multi-frame message with extended addressing
        // FF: [addr, 0x10, len_low, data...] - total 15 bytes
        // CF: [addr, 0x21, data...]
        let ff_data = vec![0x55, 0x10, 0x0F, 0x01, 0x02, 0x03, 0x04, 0x05]; // FF with 5 bytes data
        let cf1_data = vec![0x55, 0x21, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B]; // CF SN=1 with 6 bytes
        let cf2_data = vec![0x55, 0x22, 0x0C, 0x0D, 0x0E, 0x0F, 0xCC, 0xCC]; // CF SN=2 with remaining

        let ff_msg = CanMessage::new_standard(0x7E8, &ff_data).unwrap();
        let cf1_msg = CanMessage::new_standard(0x7E8, &cf1_data).unwrap();
        let cf2_msg = CanMessage::new_standard(0x7E8, &cf2_data).unwrap();

        let backend = create_backend_with_messages(vec![ff_msg, cf1_msg, cf2_msg]);

        let config = IsoTpConfig::builder()
            .tx_id(0x7E0)
            .rx_id(0x7E8)
            .addressing_mode(AddressingMode::Extended {
                target_address: 0x55,
            })
            .timeout(Duration::from_millis(500))
            .build()
            .unwrap();

        let mut channel = IsoTpChannel::new(backend, config).unwrap();

        let result = channel.receive().await;
        assert!(result.is_ok());

        let received = result.unwrap();
        assert_eq!(received.len(), 15);
        assert_eq!(
            received,
            vec![
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
                0x0F
            ]
        );
    });
}

#[test]
fn test_normal_addressing_unchanged() {
    run_local(async {
        // Verify normal addressing still works without address byte
        let sf_data = vec![0x03, 0x50, 0x01, 0x00, 0xCC, 0xCC, 0xCC, 0xCC];
        let sf_msg = CanMessage::new_standard(0x7E8, &sf_data).unwrap();
        let backend = create_backend_with_messages(vec![sf_msg]);

        let config = IsoTpConfig::builder()
            .tx_id(0x7E0)
            .rx_id(0x7E8)
            .addressing_mode(AddressingMode::Normal) // Explicit normal mode
            .timeout(Duration::from_millis(100))
            .build()
            .unwrap();

        let mut channel = IsoTpChannel::new(backend, config).unwrap();

        let result = channel.receive().await;
        assert!(result.is_ok());

        let received = result.unwrap();
        assert_eq!(received, vec![0x50, 0x01, 0x00]);
    });
}
