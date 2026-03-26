//! `LibTSCAN` CAN backend implementation.

use canlink_hal::{
    BackendConfig, BackendFactory, BackendVersion, CanBackend, CanError, CanId, CanMessage,
    CanResult, HardwareCapability, Timestamp, TimestampPrecision,
};
use canlink_tscan_sys::{
    finalize_lib_tscan, initialize_lib_tscan, tscan_config_can_by_baudrate, tscan_connect,
    tscan_disconnect_by_handle, tscan_get_can_channel_count, tscan_get_device_info,
    tscan_scan_devices, tscan_transmit_can_async, tscan_transmit_canfd_async,
    tsfifo_clear_can_receive_buffers, tsfifo_receive_can_msgs, tsfifo_receive_canfd_msgs, types::*,
    ONLY_RX_MESSAGES,
};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr;

use crate::config::TscanDaemonConfig;
use crate::convert::{from_tlibcan, from_tlibcanfd, to_tlibcan, to_tlibcanfd};
use crate::daemon::client::{DaemonClient, InitParams};
use crate::daemon::{CanFdFrame, CanFrame, ConnectResult, Op, RecvCanResult, RecvCanfdResult};
use crate::error::check_error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BackendState {
    Uninitialized,
    Ready,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BackendMode {
    Direct,
    Daemon,
}

/// CAN backend implementation for TOSUN `TSMaster` devices.
pub struct TSCanBackend {
    state: BackendState,
    mode: BackendMode,
    daemon: Option<DaemonClient>,
    device_handle: usize,
    channel_count: u8,
    opened_channels: u8,
    supports_canfd: bool,
    device_serial: Option<String>,
    recv_timeout_ms: u64,
}

impl TSCanBackend {
    /// Creates a backend instance in the uninitialized state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: BackendState::Uninitialized,
            mode: BackendMode::Direct,
            daemon: None,
            device_handle: 0,
            channel_count: 0,
            opened_channels: 0,
            supports_canfd: false,
            device_serial: None,
            recv_timeout_ms: 0,
        }
    }

    fn check_state(&self, expected: BackendState) -> CanResult<()> {
        if self.state != expected {
            return Err(CanError::InvalidState {
                expected: format!("{expected:?}"),
                current: format!("{:?}", self.state),
            });
        }
        Ok(())
    }

    fn clear_runtime_state(&mut self) {
        self.device_handle = 0;
        self.channel_count = 0;
        self.opened_channels = 0;
        self.supports_canfd = false;
        self.device_serial = None;
    }

    fn connect_device_direct(&mut self) -> CanResult<()> {
        unsafe {
            let mut device_count: u32 = 0;
            check_error(tscan_scan_devices(&mut device_count))?;
            if device_count == 0 {
                return Err(CanError::DeviceNotFound {
                    device: "No TSMaster devices found".to_string(),
                });
            }

            let mut manufacturer_ptr: *const c_char = ptr::null();
            let mut product_ptr: *const c_char = ptr::null();
            let mut serial_ptr: *const c_char = ptr::null();
            check_error(tscan_get_device_info(
                0,
                &mut manufacturer_ptr,
                &mut product_ptr,
                &mut serial_ptr,
            ))?;

            if !serial_ptr.is_null() {
                self.device_serial =
                    Some(CStr::from_ptr(serial_ptr).to_string_lossy().into_owned());
            }

            let serial_cstr = if let Some(ref serial) = self.device_serial {
                std::ffi::CString::new(serial.as_str()).map_err(|err| CanError::InvalidFormat {
                    reason: format!("invalid serial: {err}"),
                })?
            } else {
                return Err(CanError::DeviceNotFound {
                    device: "Failed to get device serial".to_string(),
                });
            };

            check_error(tscan_connect(serial_cstr.as_ptr(), &mut self.device_handle))?;

            let mut channel_count: s32 = 0;
            let mut is_canfd_supported: bool = false;
            check_error(tscan_get_can_channel_count(
                self.device_handle,
                &mut channel_count,
                &mut is_canfd_supported,
            ))?;
            self.channel_count = channel_count as u8;
            self.supports_canfd = is_canfd_supported;
            Ok(())
        }
    }

    fn disconnect_device_direct(&mut self) -> CanResult<()> {
        if self.device_handle != 0 {
            unsafe {
                check_error(tscan_disconnect_by_handle(self.device_handle))?;
            }
            self.device_handle = 0;
        }
        Ok(())
    }

    fn daemon_mut(&mut self) -> CanResult<&mut DaemonClient> {
        self.daemon.as_mut().ok_or(CanError::InvalidState {
            expected: "daemon initialized".to_string(),
            current: "daemon unavailable".to_string(),
        })
    }

    fn daemon_handle(&self) -> CanResult<u64> {
        self.daemon
            .as_ref()
            .and_then(|d| d.cache().handle)
            .ok_or(CanError::InvalidState {
                expected: "connected daemon handle".to_string(),
                current: "no daemon handle".to_string(),
            })
    }
}

impl Default for TSCanBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CanBackend for TSCanBackend {
    fn initialize(&mut self, config: &BackendConfig) -> CanResult<()> {
        self.check_state(BackendState::Uninitialized)?;

        let daemon_cfg = TscanDaemonConfig::resolve(config)?;
        self.recv_timeout_ms = daemon_cfg.recv_timeout_ms;

        if daemon_cfg.use_daemon {
            let mut daemon = DaemonClient::connect(&daemon_cfg, InitParams::default())?;
            let connected = daemon.request_auto(Op::Connect {
                serial: String::new(),
            })?;
            let info: ConnectResult = connected.decode_data()?;

            self.mode = BackendMode::Daemon;
            self.daemon = Some(daemon);
            self.device_handle = info.handle as usize;
            self.channel_count = info.channel_count;
            self.supports_canfd = info.supports_canfd;
            self.device_serial = Some(info.serial);
        } else {
            unsafe {
                initialize_lib_tscan(true, false, true);
            }
            self.connect_device_direct()?;
            self.mode = BackendMode::Direct;
            self.daemon = None;
        }

        self.state = BackendState::Ready;
        Ok(())
    }

    fn close(&mut self) -> CanResult<()> {
        if self.state == BackendState::Closed {
            return Ok(());
        }

        for channel in 0..self.channel_count {
            if (self.opened_channels & (1 << channel)) != 0 {
                let _ = self.close_channel(channel);
            }
        }

        match self.mode {
            BackendMode::Direct => {
                self.disconnect_device_direct()?;
                unsafe {
                    finalize_lib_tscan();
                }
            }
            BackendMode::Daemon => {
                let daemon_handle = self.daemon_handle().ok();
                if let Some(daemon) = self.daemon.as_mut() {
                    if let Some(handle) = daemon_handle {
                        let _ = daemon.request_auto(Op::DisconnectByHandle { handle });
                    }
                    let _ = daemon.request_auto(Op::Finalize);
                    daemon.shutdown();
                }
                self.daemon = None;
            }
        }

        self.clear_runtime_state();
        self.state = BackendState::Closed;
        Ok(())
    }

    fn get_capability(&self) -> CanResult<HardwareCapability> {
        self.check_state(BackendState::Ready)?;
        Ok(HardwareCapability {
            channel_count: self.channel_count,
            supports_canfd: self.supports_canfd,
            max_bitrate: 1_000_000,
            supported_bitrates: vec![125_000, 250_000, 500_000, 1_000_000],
            filter_count: 0,
            timestamp_precision: TimestampPrecision::Microsecond,
        })
    }

    fn send_message(&mut self, message: &CanMessage) -> CanResult<()> {
        self.check_state(BackendState::Ready)?;

        if self.opened_channels == 0 {
            return Err(CanError::ChannelNotOpen { channel: 0 });
        }

        let channel = (0..self.channel_count)
            .find(|&ch| (self.opened_channels & (1 << ch)) != 0)
            .ok_or(CanError::ChannelNotOpen { channel: 0 })?;

        match self.mode {
            BackendMode::Direct => {
                if message.is_fd() {
                    if !self.supports_canfd {
                        return Err(CanError::UnsupportedFeature {
                            feature: "CAN-FD".to_string(),
                        });
                    }
                    let libcanfd_msg = to_tlibcanfd(message, channel)?;
                    unsafe {
                        check_error(tscan_transmit_canfd_async(
                            self.device_handle,
                            &libcanfd_msg,
                        ))?;
                    }
                } else {
                    let libcan_msg = to_tlibcan(message, channel)?;
                    unsafe {
                        check_error(tscan_transmit_can_async(self.device_handle, &libcan_msg))?;
                    }
                }
            }
            BackendMode::Daemon => {
                let supports_canfd = self.supports_canfd;
                let handle = self.daemon_handle()?;
                if message.is_fd() {
                    if !supports_canfd {
                        return Err(CanError::UnsupportedFeature {
                            feature: "CAN-FD".to_string(),
                        });
                    }
                    let daemon = self.daemon_mut()?;
                    daemon.request_auto(Op::SendCanfd {
                        handle,
                        channel,
                        id: message.id().raw(),
                        is_ext: message.id().is_extended(),
                        brs: message.is_brs(),
                        esi: message.is_esi(),
                        data: message.data().to_vec(),
                    })?;
                } else {
                    let daemon = self.daemon_mut()?;
                    daemon.request_auto(Op::SendCan {
                        handle,
                        channel,
                        id: message.id().raw(),
                        is_ext: message.id().is_extended(),
                        data: message.data().to_vec(),
                    })?;
                }
            }
        }

        Ok(())
    }

    fn receive_message(&mut self) -> CanResult<Option<CanMessage>> {
        self.check_state(BackendState::Ready)?;
        if self.opened_channels == 0 {
            return Ok(None);
        }

        let channel = (0..self.channel_count)
            .find(|&ch| (self.opened_channels & (1 << ch)) != 0)
            .ok_or(CanError::ChannelNotOpen { channel: 0 })?;

        match self.mode {
            BackendMode::Direct => unsafe {
                if self.supports_canfd {
                    let mut canfd_buffer = [TLIBCANFD::default(); 1];
                    let mut canfd_size: s32 = 1;
                    let result = tsfifo_receive_canfd_msgs(
                        self.device_handle,
                        canfd_buffer.as_mut_ptr(),
                        &mut canfd_size,
                        channel,
                        ONLY_RX_MESSAGES,
                    );
                    if result == 0 && canfd_size > 0 {
                        return Ok(Some(from_tlibcanfd(&canfd_buffer[0])));
                    }
                }

                let mut can_buffer = [TLIBCAN::default(); 1];
                let mut can_size: s32 = 1;
                let result = tsfifo_receive_can_msgs(
                    self.device_handle,
                    can_buffer.as_mut_ptr(),
                    &mut can_size,
                    channel,
                    ONLY_RX_MESSAGES,
                );
                if result == 0 && can_size > 0 {
                    Ok(Some(from_tlibcan(&can_buffer[0])))
                } else {
                    Ok(None)
                }
            },
            BackendMode::Daemon => {
                let supports_canfd = self.supports_canfd;
                let recv_timeout_ms = self.recv_timeout_ms;
                let handle = self.daemon_handle()?;
                let daemon = self.daemon_mut()?;

                if supports_canfd {
                    let fd = daemon.request_auto(Op::RecvCanfd {
                        handle,
                        channel,
                        max_count: 1,
                        timeout_ms: recv_timeout_ms,
                    })?;
                    let recv: RecvCanfdResult = fd.decode_data()?;
                    if let Some(frame) = recv.messages.first() {
                        return Ok(Some(canfd_frame_to_message(frame)?));
                    }
                }

                let can = daemon.request_auto(Op::RecvCan {
                    handle,
                    channel,
                    max_count: 1,
                    timeout_ms: recv_timeout_ms,
                })?;
                let recv: RecvCanResult = can.decode_data()?;
                if let Some(frame) = recv.messages.first() {
                    Ok(Some(can_frame_to_message(frame)?))
                } else {
                    Ok(None)
                }
            }
        }
    }

    fn open_channel(&mut self, channel: u8) -> CanResult<()> {
        self.check_state(BackendState::Ready)?;
        if channel >= self.channel_count {
            return Err(CanError::ChannelNotFound {
                channel,
                max: self.channel_count.saturating_sub(1),
            });
        }
        if (self.opened_channels & (1 << channel)) != 0 {
            return Err(CanError::ChannelAlreadyOpen { channel });
        }

        match self.mode {
            BackendMode::Direct => unsafe {
                check_error(tscan_config_can_by_baudrate(
                    self.device_handle,
                    channel as u32,
                    500.0,
                    1,
                ))?;
                check_error(tsfifo_clear_can_receive_buffers(
                    self.device_handle,
                    channel as s32,
                ))?;
            },
            BackendMode::Daemon => {
                let handle = self.daemon_handle()?;
                self.daemon_mut()?
                    .request_auto(Op::OpenChannel { handle, channel })?;
            }
        }

        self.opened_channels |= 1 << channel;
        Ok(())
    }

    fn close_channel(&mut self, channel: u8) -> CanResult<()> {
        self.check_state(BackendState::Ready)?;
        if channel >= self.channel_count {
            return Err(CanError::ChannelNotFound {
                channel,
                max: self.channel_count.saturating_sub(1),
            });
        }
        if (self.opened_channels & (1 << channel)) == 0 {
            return Err(CanError::ChannelNotOpen { channel });
        }

        match self.mode {
            BackendMode::Direct => unsafe {
                check_error(tsfifo_clear_can_receive_buffers(
                    self.device_handle,
                    channel as s32,
                ))?;
            },
            BackendMode::Daemon => {
                let handle = self.daemon_handle()?;
                self.daemon_mut()?
                    .request_auto(Op::CloseChannel { handle, channel })?;
            }
        }

        self.opened_channels &= !(1 << channel);
        Ok(())
    }

    fn version(&self) -> BackendVersion {
        BackendVersion::new(0, 1, 0)
    }

    fn name(&self) -> &str {
        "tscan"
    }
}

impl Drop for TSCanBackend {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

/// Factory used to create [`TSCanBackend`] instances through `canlink-hal`.
pub struct TSCanBackendFactory;

impl TSCanBackendFactory {
    /// Creates a backend factory for the `tscan` backend.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for TSCanBackendFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl BackendFactory for TSCanBackendFactory {
    fn create(&self, _config: &BackendConfig) -> CanResult<Box<dyn CanBackend>> {
        Ok(Box::new(TSCanBackend::new()))
    }

    fn name(&self) -> &'static str {
        "tscan"
    }

    fn version(&self) -> BackendVersion {
        BackendVersion::new(0, 1, 0)
    }
}

fn can_frame_to_message(frame: &CanFrame) -> CanResult<CanMessage> {
    let mut message = if frame.is_ext {
        CanMessage::new_extended(frame.id, &frame.data)?
    } else {
        CanMessage::new_standard(frame.id as u16, &frame.data)?
    };
    if let Some(ts) = frame.timestamp_us {
        message.set_timestamp(Timestamp::from_micros(ts));
    }
    Ok(message)
}

fn canfd_frame_to_message(frame: &CanFdFrame) -> CanResult<CanMessage> {
    let id = if frame.is_ext {
        CanId::Extended(frame.id)
    } else {
        CanId::Standard(frame.id as u16)
    };
    let mut message = CanMessage::new_fd(id, &frame.data)?;
    if let Some(ts) = frame.timestamp_us {
        message.set_timestamp(Timestamp::from_micros(ts));
    }
    Ok(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_backend() {
        let backend = TSCanBackend::new();
        assert_eq!(backend.state, BackendState::Uninitialized);
        assert_eq!(backend.device_handle, 0);
        assert_eq!(backend.mode, BackendMode::Direct);
    }

    #[test]
    fn test_backend_name() {
        let backend = TSCanBackend::new();
        assert_eq!(backend.name(), "tscan");
    }

    #[test]
    fn test_backend_version() {
        let backend = TSCanBackend::new();
        let version = backend.version();
        assert_eq!(version.major(), 0);
        assert_eq!(version.minor(), 1);
        assert_eq!(version.patch(), 0);
    }
}
