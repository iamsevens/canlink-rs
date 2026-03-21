//! Basic LibTSCAN hardware connection test
//!
//! This example demonstrates basic LibTSCAN usage:
//! 1. Initialize library
//! 2. Scan for devices
//! 3. Connect to device
//! 4. Query capabilities
//! 5. Configure channel
//! 6. Send a test message
//! 7. Receive messages
//! 8. Cleanup
//!
//! **Requirements**:
//! - Connected LibTSCAN-compatible hardware (validated in this repository on TOSUN-related devices)
//! - libTSCAN.dll in PATH or executable directory

use canlink_tscan_sys::*;
use std::ffi::CStr;
use std::ptr;

fn main() {
    println!("🔍 LibTSCAN Hardware Connection Test\n");
    println!("=====================================\n");

    unsafe {
        // Step 1: Initialize library
        println!("1. Initializing LibTSCAN library...");
        initialize_lib_tscan(true, false, false); // Enable FIFO, no error frames, no HW time
        println!("   ✓ Library initialized\n");

        // Step 2: Scan for devices
        println!("2. Scanning for devices...");
        let mut device_count: u32 = 0;
        let result = tscan_scan_devices(&mut device_count);

        if result != 0 {
            println!("   ❌ Failed to scan devices (error code: {})", result);
            finalize_lib_tscan();
            return;
        }

        println!("   ✓ Found {} device(s)\n", device_count);

        if device_count == 0 {
            println!("   ⚠️  No devices found. Please connect a TSMaster device.");
            finalize_lib_tscan();
            return;
        }

        // Step 3: Get device info
        println!("3. Getting device information...");
        for i in 0..device_count {
            let mut manufacturer: *const i8 = ptr::null();
            let mut product: *const i8 = ptr::null();
            let mut serial: *const i8 = ptr::null();

            let result = tscan_get_device_info(i, &mut manufacturer, &mut product, &mut serial);

            if result == 0 {
                let mfg = if !manufacturer.is_null() {
                    CStr::from_ptr(manufacturer).to_string_lossy()
                } else {
                    "Unknown".into()
                };

                let prod = if !product.is_null() {
                    CStr::from_ptr(product).to_string_lossy()
                } else {
                    "Unknown".into()
                };

                let ser = if !serial.is_null() {
                    CStr::from_ptr(serial).to_string_lossy()
                } else {
                    "Unknown".into()
                };

                println!("   Device {}: {} {} (S/N: {})", i, mfg, prod, ser);
            }
        }
        println!();

        // Step 4: Connect to default device
        println!("4. Connecting to default device...");
        let mut handle: usize = 0;
        let result = tscan_connect(ptr::null(), &mut handle);

        if result != 0 {
            println!("   ❌ Failed to connect (error code: {})", result);
            finalize_lib_tscan();
            return;
        }

        println!("   ✓ Connected (handle: 0x{:X})\n", handle);

        // Step 5: Query capabilities
        println!("5. Querying device capabilities...");
        let mut channel_count: i32 = 0;
        let mut is_canfd: bool = false;

        let result = tscan_get_can_channel_count(handle, &mut channel_count, &mut is_canfd);

        if result == 0 {
            println!("   CAN Channels: {}", channel_count);
            println!("   CAN-FD Support: {}", if is_canfd { "Yes" } else { "No" });
        } else {
            println!(
                "   ⚠️  Failed to query capabilities (error code: {})",
                result
            );
        }

        let mut device_type: i32 = 0;
        let mut device_name: *const i8 = ptr::null();
        let result = tscan_get_device_type(handle, &mut device_type, &mut device_name);

        if result == 0 {
            let name = if !device_name.is_null() {
                CStr::from_ptr(device_name).to_string_lossy()
            } else {
                "Unknown".into()
            };
            println!("   Device Type: {} ({})", device_type, name);
        }
        println!();

        // Step 6: Configure CAN channel
        println!("6. Configuring CAN channel 0 (500 kbps)...");
        let result = tscan_config_can_by_baudrate(handle, CHN1, 500.0, 1); // 500 kbps, 120Ω on

        if result != 0 {
            println!("   ❌ Failed to configure channel (error code: {})", result);
        } else {
            println!("   ✓ Channel configured\n");
        }

        // Step 7: Send a test message
        println!("7. Sending test CAN message...");
        let msg = TLIBCAN {
            FIdxChn: 0,
            FProperties: MASK_CANPROP_DIR_TX, // TX, standard, data frame
            FDLC: 8,
            FReserved: 0,
            FIdentifier: 0x123,
            FTimeUs: 0,
            FData: [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
        };

        let result = tscan_transmit_can_async(handle, &msg);

        if result != 0 {
            println!("   ❌ Failed to send message (error code: {})", result);
        } else {
            // Copy fields to avoid unaligned reference
            let id = msg.FIdentifier;
            let dlc = msg.FDLC;
            let data = msg.FData;
            println!(
                "   ✓ Message sent: ID=0x{:03X}, DLC={}, Data={:02X?}",
                id, dlc, &data
            );
        }
        println!();

        // Step 8: Try to receive messages
        println!("8. Receiving messages (5 seconds)...");
        let mut rx_buffer: [TLIBCAN; 100] = [TLIBCAN::default(); 100];
        let mut received_count = 0;

        for _ in 0..50 {
            // Try for 5 seconds (50 * 100ms)
            let mut buffer_size: i32 = 100;
            let result = tsfifo_receive_can_msgs(
                handle,
                rx_buffer.as_mut_ptr(),
                &mut buffer_size,
                0, // Channel 0
                ONLY_RX_MESSAGES,
            );

            if result == 0 && buffer_size > 0 {
                for msg in rx_buffer.iter().take(buffer_size as usize) {
                    let dir = if msg.FProperties & MASK_CANPROP_DIR_TX != 0 {
                        "TX"
                    } else {
                        "RX"
                    };
                    let frame_type = if msg.FProperties & MASK_CANPROP_EXTEND != 0 {
                        "Ext"
                    } else {
                        "Std"
                    };

                    // Copy fields to avoid unaligned reference
                    let id = msg.FIdentifier;
                    let dlc = msg.FDLC;
                    let data_len = dlc.min(8) as usize;
                    let mut data_slice = [0u8; 8];
                    data_slice[..data_len].copy_from_slice(&msg.FData[..data_len]);

                    println!(
                        "   {} {} ID=0x{:03X}, DLC={}, Data={:02X?}",
                        dir,
                        frame_type,
                        id,
                        dlc,
                        &data_slice[..data_len]
                    );
                    received_count += 1;
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        println!("   Total received: {} messages\n", received_count);

        // Step 9: Cleanup
        println!("9. Cleaning up...");
        tscan_disconnect_by_handle(handle);
        println!("   ✓ Disconnected");

        finalize_lib_tscan();
        println!("   ✓ Library finalized\n");

        println!("=====================================");
        println!("✅ Test completed successfully!");
    }
}
