//! Low-level FFI function declarations for LibTSCAN
//!
//! This module contains the raw C function bindings.
//! All functions use `extern "system"` which maps to `__stdcall` on Windows.

use crate::types::*;
use std::os::raw::c_char;

#[link(name = "libTSCAN")]
extern "system" {
    // ========================================================================
    // Library Initialization
    // ========================================================================

    /// Initialize the LibTSCAN library
    ///
    /// # Parameters
    /// - `AEnableFIFO`: Enable FIFO mode for message reception
    /// - `AEnableErrorFrame`: Enable error frame reception
    /// - `AUseHWTime`: Use hardware timestamp
    ///
    /// # Safety
    /// Must be called before any other LibTSCAN functions.
    /// Should only be called once per process.
    pub fn initialize_lib_tscan(AEnableFIFO: bool, AEnableErrorFrame: bool, AUseHWTime: bool);

    /// Finalize the LibTSCAN library
    ///
    /// # Safety
    /// Should be called when done using LibTSCAN.
    /// No LibTSCAN functions should be called after this.
    pub fn finalize_lib_tscan();

    // ========================================================================
    // Device Discovery and Connection
    // ========================================================================

    /// Scan for available TSMaster devices
    ///
    /// # Parameters
    /// - `ADeviceCount`: Output parameter for device count
    ///
    /// # Returns
    /// 0 on success, error code otherwise
    pub fn tscan_scan_devices(ADeviceCount: *mut u32) -> u32;

    /// Get device information
    ///
    /// # Parameters
    /// - `ADeviceIndex`: Device index (0-based)
    /// - `AFManufacturer`: Output pointer to manufacturer string
    /// - `AFProduct`: Output pointer to product string
    /// - `AFSerial`: Output pointer to serial number string
    ///
    /// # Returns
    /// 0 on success, error code otherwise
    ///
    /// # Safety
    /// Returned strings are owned by the library. Do not free them.
    pub fn tscan_get_device_info(
        ADeviceIndex: u32,
        AFManufacturer: *mut *const c_char,
        AFProduct: *mut *const c_char,
        AFSerial: *mut *const c_char,
    ) -> u32;

    /// Connect to a device
    ///
    /// # Parameters
    /// - `ADeviceSerial`: Device serial number (NULL for default device)
    /// - `AHandle`: Output parameter for device handle
    ///
    /// # Returns
    /// 0 on success, error code otherwise
    pub fn tscan_connect(ADeviceSerial: *const c_char, AHandle: *mut usize) -> u32;

    /// Disconnect from a device by handle
    ///
    /// # Parameters
    /// - `ADeviceHandle`: Device handle
    ///
    /// # Returns
    /// 0 on success, error code otherwise
    pub fn tscan_disconnect_by_handle(ADeviceHandle: usize) -> u32;

    /// Disconnect all devices
    ///
    /// # Returns
    /// 0 on success, error code otherwise
    pub fn tscan_disconnect_all_devices() -> u32;

    // ========================================================================
    // Hardware Capability Query
    // ========================================================================

    /// Get CAN channel count
    ///
    /// # Parameters
    /// - `ADeviceHandle`: Device handle
    /// - `AChnCount`: Output parameter for channel count
    /// - `AIsFDCAN`: Output parameter indicating CAN-FD support
    ///
    /// # Returns
    /// 0 on success, error code otherwise
    pub fn tscan_get_can_channel_count(
        ADeviceHandle: usize,
        AChnCount: *mut s32,
        AIsFDCAN: *mut bool,
    ) -> u32;

    /// Get device type
    ///
    /// # Parameters
    /// - `ADeviceHandle`: Device handle
    /// - `ADeviceType`: Output parameter for device type
    /// - `ADeviceName`: Output pointer to device name string
    ///
    /// # Returns
    /// 0 on success, error code otherwise
    pub fn tscan_get_device_type(
        ADeviceHandle: usize,
        ADeviceType: *mut s32,
        ADeviceName: *mut *const c_char,
    ) -> u32;

    // ========================================================================
    // Channel Configuration
    // ========================================================================

    /// Configure CAN channel baudrate
    ///
    /// # Parameters
    /// - `ADeviceHandle`: Device handle
    /// - `AChnIdx`: Channel index (0-based)
    /// - `ARateKbps`: Baudrate in kbps (e.g., 500.0 for 500 kbps)
    /// - `A120OhmConnected`: 1 if 120Ω termination is connected, 0 otherwise
    ///
    /// # Returns
    /// 0 on success, error code otherwise
    pub fn tscan_config_can_by_baudrate(
        ADeviceHandle: usize,
        AChnIdx: u32,
        ARateKbps: f64,
        A120OhmConnected: u32,
    ) -> u32;

    /// Configure CAN-FD channel baudrate
    ///
    /// # Parameters
    /// - `ADeviceHandle`: Device handle
    /// - `AChnIdx`: Channel index (0-based)
    /// - `AArbRateKbps`: Arbitration phase baudrate in kbps
    /// - `ADataRateKbps`: Data phase baudrate in kbps
    /// - `AControllerType`: Controller type (ISO/Non-ISO)
    /// - `AControllerMode`: Controller mode (Normal/ACKOff/Restricted)
    /// - `A120OhmConnected`: 1 if 120Ω termination is connected, 0 otherwise
    ///
    /// # Returns
    /// 0 on success, error code otherwise
    pub fn tscan_config_canfd_by_baudrate(
        ADeviceHandle: usize,
        AChnIdx: s32,
        AArbRateKbps: f64,
        ADataRateKbps: f64,
        AControllerType: TLIBCANFDControllerType,
        AControllerMode: TLIBCANFDControllerMode,
        A120OhmConnected: s32,
    ) -> u32;

    // ========================================================================
    // Message Transmission
    // ========================================================================

    /// Transmit CAN message synchronously (blocking)
    ///
    /// # Parameters
    /// - `ADeviceHandle`: Device handle
    /// - `ACAN`: Pointer to CAN message
    /// - `ATimeoutMS`: Timeout in milliseconds
    ///
    /// # Returns
    /// 0 on success, error code otherwise
    pub fn tscan_transmit_can_sync(
        ADeviceHandle: usize,
        ACAN: *const TLIBCAN,
        ATimeoutMS: u32,
    ) -> u32;

    /// Transmit CAN message asynchronously (non-blocking)
    ///
    /// # Parameters
    /// - `ADeviceHandle`: Device handle
    /// - `ACAN`: Pointer to CAN message
    ///
    /// # Returns
    /// 0 on success, error code otherwise
    pub fn tscan_transmit_can_async(ADeviceHandle: usize, ACAN: *const TLIBCAN) -> u32;

    /// Transmit CAN-FD message synchronously (blocking)
    ///
    /// # Parameters
    /// - `ADeviceHandle`: Device handle
    /// - `ACANFD`: Pointer to CAN-FD message
    /// - `ATimeoutMS`: Timeout in milliseconds
    ///
    /// # Returns
    /// 0 on success, error code otherwise
    pub fn tscan_transmit_canfd_sync(
        ADeviceHandle: usize,
        ACANFD: *const TLIBCANFD,
        ATimeoutMS: u32,
    ) -> u32;

    /// Transmit CAN-FD message asynchronously (non-blocking)
    ///
    /// # Parameters
    /// - `ADeviceHandle`: Device handle
    /// - `ACANFD`: Pointer to CAN-FD message
    ///
    /// # Returns
    /// 0 on success, error code otherwise
    pub fn tscan_transmit_canfd_async(ADeviceHandle: usize, ACANFD: *const TLIBCANFD) -> u32;

    // ========================================================================
    // Message Reception
    // ========================================================================

    /// Receive CAN messages from FIFO
    ///
    /// # Parameters
    /// - `ADeviceHandle`: Device handle
    /// - `ACANBuffers`: Pointer to buffer for received messages
    /// - `ACANBufferSize`: Input: buffer size, Output: actual message count
    /// - `AChn`: Channel index (0-based)
    /// - `ARXTX`: 0=RX only, 1=RX+TX
    ///
    /// # Returns
    /// 0 on success, error code otherwise
    pub fn tsfifo_receive_can_msgs(
        ADeviceHandle: usize,
        ACANBuffers: *mut TLIBCAN,
        ACANBufferSize: *mut s32,
        AChn: u8,
        ARXTX: u8,
    ) -> u32;

    /// Receive CAN-FD messages from FIFO
    ///
    /// # Parameters
    /// - `ADeviceHandle`: Device handle
    /// - `ACANBuffers`: Pointer to buffer for received messages
    /// - `ACANBufferSize`: Input: buffer size, Output: actual message count
    /// - `AChn`: Channel index (0-based)
    /// - `ARXTX`: 0=RX only, 1=RX+TX
    ///
    /// # Returns
    /// 0 on success, error code otherwise
    pub fn tsfifo_receive_canfd_msgs(
        ADeviceHandle: usize,
        ACANBuffers: *mut TLIBCANFD,
        ACANBufferSize: *mut s32,
        AChn: u8,
        ARXTX: u8,
    ) -> u32;

    /// Clear CAN receive buffers
    ///
    /// # Parameters
    /// - `ADeviceHandle`: Device handle
    /// - `AIdxChn`: Channel index (0-based)
    ///
    /// # Returns
    /// 0 on success, error code otherwise
    pub fn tsfifo_clear_can_receive_buffers(ADeviceHandle: usize, AIdxChn: s32) -> u32;

    /// Clear CAN-FD receive buffers
    ///
    /// # Parameters
    /// - `ADeviceHandle`: Device handle
    /// - `AIdxChn`: Channel index (0-based)
    ///
    /// # Returns
    /// 0 on success, error code otherwise
    pub fn tsfifo_clear_canfd_receive_buffers(ADeviceHandle: usize, AIdxChn: s32) -> u32;

    /// Read CAN buffer frame count
    ///
    /// # Parameters
    /// - `ADeviceHandle`: Device handle
    /// - `AIdxChn`: Channel index (0-based)
    /// - `ACount`: Output parameter for frame count
    ///
    /// # Returns
    /// 0 on success, error code otherwise
    pub fn tsfifo_read_can_buffer_frame_count(
        ADeviceHandle: usize,
        AIdxChn: s32,
        ACount: *mut s32,
    ) -> s32;

    // ========================================================================
    // Event Callbacks (Optional)
    // ========================================================================

    /// Register CAN message receive callback
    ///
    /// # Parameters
    /// - `ADeviceHandle`: Device handle
    /// - `ACallback`: Callback function
    ///
    /// # Returns
    /// 0 on success, error code otherwise
    pub fn tscan_register_event_can(ADeviceHandle: usize, ACallback: TCANQueueEvent_Win32) -> u32;

    /// Unregister CAN message receive callback
    ///
    /// # Parameters
    /// - `ADeviceHandle`: Device handle
    /// - `ACallback`: Callback function
    ///
    /// # Returns
    /// 0 on success, error code otherwise
    pub fn tscan_unregister_event_can(ADeviceHandle: usize, ACallback: TCANQueueEvent_Win32)
        -> u32;

    /// Register CAN-FD message receive callback
    ///
    /// # Parameters
    /// - `ADeviceHandle`: Device handle
    /// - `ACallback`: Callback function
    ///
    /// # Returns
    /// 0 on success, error code otherwise
    pub fn tscan_register_event_canfd(
        ADeviceHandle: usize,
        ACallback: TCANFDQueueEvent_Win32,
    ) -> u32;

    /// Unregister CAN-FD message receive callback
    ///
    /// # Parameters
    /// - `ADeviceHandle`: Device handle
    /// - `ACallback`: Callback function
    ///
    /// # Returns
    /// 0 on success, error code otherwise
    pub fn tscan_unregister_event_canfd(
        ADeviceHandle: usize,
        ACallback: TCANFDQueueEvent_Win32,
    ) -> u32;

    // ========================================================================
    // Error Handling
    // ========================================================================

    /// Get error description
    ///
    /// # Parameters
    /// - `ACode`: Error code
    /// - `ADesc`: Output pointer to error description string
    ///
    /// # Returns
    /// 0 on success, error code otherwise
    ///
    /// # Safety
    /// Returned string is owned by the library. Do not free it.
    pub fn tscan_get_error_description(ACode: u32, ADesc: *mut *const c_char) -> u32;
}
