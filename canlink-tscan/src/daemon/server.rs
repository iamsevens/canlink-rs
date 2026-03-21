use super::codec::{read_frame, write_frame};
use super::protocol::{
    CanFdFrame, CanFrame, CapabilityResult, ConnectResult, DeviceInfo, ErrorCode, HelloAck, Op,
    RecvCanResult, RecvCanfdResult, Request, Response, ScanResult,
};
use canlink_hal::CanMessage;
use canlink_tscan_sys::{
    finalize_lib_tscan, initialize_lib_tscan, tscan_config_can_by_baudrate,
    tscan_config_canfd_by_baudrate, tscan_connect, tscan_disconnect_all_devices,
    tscan_disconnect_by_handle, tscan_get_can_channel_count, tscan_get_device_info,
    tscan_scan_devices, tscan_transmit_can_async, tscan_transmit_canfd_async,
    tsfifo_clear_can_receive_buffers, tsfifo_receive_can_msgs, tsfifo_receive_canfd_msgs,
    TLIBCANFDControllerMode, TLIBCANFDControllerType, ONLY_RX_MESSAGES, TLIBCAN, TLIBCANFD,
};
use std::collections::HashSet;
use std::ffi::{CStr, CString};
use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::os::raw::c_char;
use std::thread;
use std::time::Duration;

const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Default)]
struct ServerState {
    initialized: bool,
    delayed_once_ops: HashSet<String>,
    exited_once_ops: HashSet<String>,
}

pub fn run_server() -> io::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();
    run_server_with_io(&mut input, &mut output)
}

pub fn run_server_with_io<R: Read, W: Write>(input: &mut R, output: &mut W) -> io::Result<()> {
    let mut state = ServerState::default();
    loop {
        let request: Request = match read_frame(input) {
            Ok(req) => req,
            Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(err) => return Err(err),
        };
        let op_name = op_name(&request.op);
        append_trace(format!("OP:{op_name}"))?;
        let should_exit = matches!(request.op, Op::Finalize);
        let response = handle_request(&mut state, request);
        if matches!(
            maybe_inject_fault(&mut state, op_name)?,
            FaultAction::ReturnEarly
        ) {
            return Ok(());
        }
        write_frame(output, &response)?;
        if should_exit {
            return Ok(());
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FaultAction {
    Continue,
    ReturnEarly,
}

fn maybe_inject_fault(state: &mut ServerState, op_name: &str) -> io::Result<FaultAction> {
    if should_fire_once(op_name, "DELAY_ON_OP_ONCE", &mut state.delayed_once_ops) {
        let delay_ms = std::env::var("DELAY_MS")
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok())
            .unwrap_or(0);
        append_trace(format!("INJECT_DELAY_ONCE:{op_name}:{delay_ms}"))?;
        thread::sleep(Duration::from_millis(delay_ms));
    }

    if should_fire_once(op_name, "EXIT_ON_OP_ONCE", &mut state.exited_once_ops) {
        append_trace(format!("INJECT_EXIT_ONCE:{op_name}"))?;
        return Ok(FaultAction::ReturnEarly);
    }

    Ok(FaultAction::Continue)
}

fn should_fire_once(op_name: &str, env_name: &str, seen: &mut HashSet<String>) -> bool {
    let configured = match std::env::var(env_name) {
        Ok(value) => value,
        Err(_) => return false,
    };
    if configured.trim() != op_name {
        return false;
    }
    seen.insert(op_name.to_string())
}

fn append_trace(line: String) -> io::Result<()> {
    let Ok(path) = std::env::var("TRACE_PATH") else {
        return Ok(());
    };
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{line}")?;
    Ok(())
}

fn op_name(op: &Op) -> &'static str {
    match op {
        Op::Hello { .. } => "HELLO",
        Op::InitLib { .. } => "INIT_LIB",
        Op::Scan => "SCAN",
        Op::GetDeviceInfo { .. } => "GET_DEVICE_INFO",
        Op::Connect { .. } => "CONNECT",
        Op::DisconnectByHandle { .. } => "DISCONNECT_BY_HANDLE",
        Op::DisconnectAll => "DISCONNECT_ALL",
        Op::OpenChannel { .. } => "OPEN_CHANNEL",
        Op::CloseChannel { .. } => "CLOSE_CHANNEL",
        Op::ConfigCanBaudrate { .. } => "CONFIG_CAN_BAUDRATE",
        Op::ConfigCanfdBaudrate { .. } => "CONFIG_CANFD_BAUDRATE",
        Op::SendCan { .. } => "SEND_CAN",
        Op::SendCanfd { .. } => "SEND_CANFD",
        Op::RecvCan { .. } => "RECV_CAN",
        Op::RecvCanfd { .. } => "RECV_CANFD",
        Op::GetCapability { .. } => "GET_CAPABILITY",
        Op::Finalize => "FINALIZE",
    }
}

fn handle_request(state: &mut ServerState, request: Request) -> Response {
    let id = request.id;
    match request.op {
        Op::Hello {
            protocol_version, ..
        } => {
            if protocol_version != PROTOCOL_VERSION {
                return Response::error(
                    id,
                    ErrorCode::ProtocolError as u32,
                    format!(
                        "protocol mismatch: client={protocol_version}, daemon={PROTOCOL_VERSION}"
                    ),
                );
            }
            Response::ok_data(
                id,
                &HelloAck {
                    protocol_version: PROTOCOL_VERSION,
                    daemon_version: env!("CARGO_PKG_VERSION").to_string(),
                },
            )
        }
        Op::InitLib {
            enable_fifo,
            enable_error_frame,
            use_hw_time,
        } => unsafe {
            initialize_lib_tscan(enable_fifo, enable_error_frame, use_hw_time);
            state.initialized = true;
            Response::ok_empty(id)
        },
        Op::Scan => {
            let mut device_count = 0u32;
            let rc = unsafe { tscan_scan_devices(&mut device_count) };
            if rc != 0 {
                return lib_error(id, "tscan_scan_devices", rc);
            }

            let mut devices = Vec::with_capacity(device_count as usize);
            for index in 0..device_count {
                match query_device_info(index) {
                    Ok(info) => devices.push(info),
                    Err(rc) => return lib_error(id, "tscan_get_device_info", rc),
                }
            }
            Response::ok_data(id, &ScanResult { devices })
        }
        Op::GetDeviceInfo { index } => match query_device_info(index) {
            Ok(info) => Response::ok_data(id, &info),
            Err(rc) => lib_error(id, "tscan_get_device_info", rc),
        },
        Op::Connect { serial } => {
            let resolved_serial = if serial.is_empty() {
                let mut device_count = 0u32;
                let rc = unsafe { tscan_scan_devices(&mut device_count) };
                if rc != 0 {
                    return lib_error(id, "tscan_scan_devices", rc);
                }
                if device_count == 0 {
                    return Response::error(
                        id,
                        ErrorCode::NoDevice as u32,
                        "no device detected by tscan_scan_devices",
                    );
                }
                match query_device_info(0) {
                    Ok(info) if !info.serial.is_empty() => info.serial,
                    Ok(_) => {
                        return Response::error(
                            id,
                            ErrorCode::InvalidParams as u32,
                            "device serial is empty",
                        );
                    }
                    Err(rc) => return lib_error(id, "tscan_get_device_info", rc),
                }
            } else {
                serial
            };

            let mut handle = 0usize;
            let rc = unsafe {
                let serial_c = match CString::new(resolved_serial.as_str()) {
                    Ok(v) => v,
                    Err(err) => {
                        return Response::error(
                            id,
                            ErrorCode::InvalidParams as u32,
                            format!("invalid serial: {err}"),
                        );
                    }
                };
                tscan_connect(serial_c.as_ptr(), &mut handle)
            };
            if rc != 0 {
                return lib_error(id, "tscan_connect", rc);
            }

            let mut channel_count = 0i32;
            let mut supports_canfd = false;
            let rc = unsafe {
                tscan_get_can_channel_count(handle, &mut channel_count, &mut supports_canfd)
            };
            if rc != 0 {
                return lib_error(id, "tscan_get_can_channel_count", rc);
            }

            let serial_value = resolved_serial;

            Response::ok_data(
                id,
                &ConnectResult {
                    handle: handle as u64,
                    channel_count: channel_count as u8,
                    supports_canfd,
                    serial: serial_value,
                },
            )
        }
        Op::DisconnectByHandle { handle } => {
            let rc = unsafe { tscan_disconnect_by_handle(handle as usize) };
            if rc != 0 {
                return lib_error(id, "tscan_disconnect_by_handle", rc);
            }
            Response::ok_empty(id)
        }
        Op::DisconnectAll => {
            let rc = unsafe { tscan_disconnect_all_devices() };
            if rc != 0 {
                return lib_error(id, "tscan_disconnect_all_devices", rc);
            }
            Response::ok_empty(id)
        }
        Op::OpenChannel { handle, channel } => {
            let rc =
                unsafe { tscan_config_can_by_baudrate(handle as usize, channel as u32, 500.0, 1) };
            if rc != 0 {
                return lib_error(id, "tscan_config_can_by_baudrate", rc);
            }
            let rc = unsafe { tsfifo_clear_can_receive_buffers(handle as usize, channel as i32) };
            if rc != 0 {
                return lib_error(id, "tsfifo_clear_can_receive_buffers", rc);
            }
            Response::ok_empty(id)
        }
        Op::CloseChannel { handle, channel } => {
            let rc = unsafe { tsfifo_clear_can_receive_buffers(handle as usize, channel as i32) };
            if rc != 0 {
                return lib_error(id, "tsfifo_clear_can_receive_buffers", rc);
            }
            Response::ok_empty(id)
        }
        Op::ConfigCanBaudrate {
            handle,
            channel,
            rate_kbps,
            term,
        } => {
            let rc = unsafe {
                tscan_config_can_by_baudrate(
                    handle as usize,
                    channel as u32,
                    rate_kbps,
                    u32::from(term),
                )
            };
            if rc != 0 {
                return lib_error(id, "tscan_config_can_by_baudrate", rc);
            }
            Response::ok_empty(id)
        }
        Op::ConfigCanfdBaudrate {
            handle,
            channel,
            arb_kbps,
            data_kbps,
            ctrl_type,
            ctrl_mode,
            term,
        } => {
            let controller_type = to_controller_type(ctrl_type);
            let controller_mode = to_controller_mode(ctrl_mode);
            let rc = unsafe {
                tscan_config_canfd_by_baudrate(
                    handle as usize,
                    i32::from(channel),
                    arb_kbps,
                    data_kbps,
                    controller_type,
                    controller_mode,
                    i32::from(term),
                )
            };
            if rc != 0 {
                return lib_error(id, "tscan_config_canfd_by_baudrate", rc);
            }
            Response::ok_empty(id)
        }
        Op::SendCan {
            handle,
            channel,
            id: can_id,
            is_ext,
            data,
        } => {
            let message = if is_ext {
                match CanMessage::new_extended(can_id, &data) {
                    Ok(v) => v,
                    Err(err) => {
                        return Response::error(
                            id,
                            ErrorCode::InvalidParams as u32,
                            err.to_string(),
                        )
                    }
                }
            } else {
                match CanMessage::new_standard(can_id as u16, &data) {
                    Ok(v) => v,
                    Err(err) => {
                        return Response::error(
                            id,
                            ErrorCode::InvalidParams as u32,
                            err.to_string(),
                        )
                    }
                }
            };
            let can = match crate::convert::to_tlibcan(&message, channel) {
                Ok(v) => v,
                Err(err) => {
                    return Response::error(id, ErrorCode::InvalidParams as u32, err.to_string())
                }
            };
            let rc = unsafe { tscan_transmit_can_async(handle as usize, &can) };
            if rc != 0 {
                return lib_error(id, "tscan_transmit_can_async", rc);
            }
            Response::ok_empty(id)
        }
        Op::SendCanfd {
            handle,
            channel,
            id: can_id,
            is_ext,
            brs,
            esi,
            data,
        } => {
            let canfd = match build_canfd_frame(channel, can_id, is_ext, brs, esi, &data) {
                Ok(v) => v,
                Err(err) => return Response::error(id, ErrorCode::InvalidParams as u32, err),
            };
            let rc = unsafe { tscan_transmit_canfd_async(handle as usize, &canfd) };
            if rc != 0 {
                return lib_error(id, "tscan_transmit_canfd_async", rc);
            }
            Response::ok_empty(id)
        }
        Op::RecvCan {
            handle,
            channel,
            max_count,
            timeout_ms: _,
        } => {
            let count = max_count.max(1) as usize;
            let mut buffer = vec![TLIBCAN::default(); count];
            let mut size = count as i32;
            let rc = unsafe {
                tsfifo_receive_can_msgs(
                    handle as usize,
                    buffer.as_mut_ptr(),
                    &mut size,
                    channel,
                    ONLY_RX_MESSAGES,
                )
            };
            if rc != 0 {
                return lib_error(id, "tsfifo_receive_can_msgs", rc);
            }
            let size = size.max(0) as usize;
            let messages = buffer[..size]
                .iter()
                .map(parse_can_frame)
                .collect::<Vec<_>>();
            Response::ok_data(id, &RecvCanResult { messages })
        }
        Op::RecvCanfd {
            handle,
            channel,
            max_count,
            timeout_ms: _,
        } => {
            let count = max_count.max(1) as usize;
            let mut buffer = vec![TLIBCANFD::default(); count];
            let mut size = count as i32;
            let rc = unsafe {
                tsfifo_receive_canfd_msgs(
                    handle as usize,
                    buffer.as_mut_ptr(),
                    &mut size,
                    channel,
                    ONLY_RX_MESSAGES,
                )
            };
            if rc != 0 {
                return lib_error(id, "tsfifo_receive_canfd_msgs", rc);
            }
            let size = size.max(0) as usize;
            let messages = buffer[..size]
                .iter()
                .map(parse_canfd_frame)
                .collect::<Vec<_>>();
            Response::ok_data(id, &RecvCanfdResult { messages })
        }
        Op::GetCapability { handle } => {
            let mut channel_count = 0i32;
            let mut supports_canfd = false;
            let rc = unsafe {
                tscan_get_can_channel_count(
                    handle as usize,
                    &mut channel_count,
                    &mut supports_canfd,
                )
            };
            if rc != 0 {
                return lib_error(id, "tscan_get_can_channel_count", rc);
            }
            Response::ok_data(
                id,
                &CapabilityResult {
                    channel_count: channel_count as u8,
                    supports_canfd,
                    max_bitrate_kbps: 1000,
                    supported_bitrates_kbps: vec![125, 250, 500, 1000],
                },
            )
        }
        Op::Finalize => unsafe {
            if state.initialized {
                finalize_lib_tscan();
            }
            state.initialized = false;
            Response::ok_empty(id)
        },
    }
}

fn query_device_info(index: u32) -> Result<DeviceInfo, u32> {
    let mut manufacturer_ptr: *const c_char = std::ptr::null();
    let mut product_ptr: *const c_char = std::ptr::null();
    let mut serial_ptr: *const c_char = std::ptr::null();
    let rc = unsafe {
        tscan_get_device_info(
            index,
            &mut manufacturer_ptr,
            &mut product_ptr,
            &mut serial_ptr,
        )
    };
    if rc != 0 {
        return Err(rc);
    }
    Ok(DeviceInfo {
        manufacturer: cstr_or_empty(manufacturer_ptr),
        product: cstr_or_empty(product_ptr),
        serial: cstr_or_empty(serial_ptr),
        device_type: 0,
    })
}

fn cstr_or_empty(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() }
}

fn lib_error(id: u64, op: &str, code: u32) -> Response {
    Response::error(
        id,
        ErrorCode::LibTscanError as u32,
        format!("{op} failed: {code}"),
    )
}

fn to_controller_type(value: u8) -> TLIBCANFDControllerType {
    match value {
        0 => TLIBCANFDControllerType::lfdtCAN,
        2 => TLIBCANFDControllerType::lfdtNonISOCAN,
        _ => TLIBCANFDControllerType::lfdtISOCAN,
    }
}

fn to_controller_mode(value: u8) -> TLIBCANFDControllerMode {
    match value {
        1 => TLIBCANFDControllerMode::lfdmACKOff,
        2 => TLIBCANFDControllerMode::lfdmRestricted,
        _ => TLIBCANFDControllerMode::lfdmNormal,
    }
}

fn build_canfd_frame(
    channel: u8,
    can_id: u32,
    is_ext: bool,
    brs: bool,
    esi: bool,
    data: &[u8],
) -> Result<TLIBCANFD, String> {
    if data.len() > 64 {
        return Err(format!("CAN-FD data too large: {}", data.len()));
    }

    let mut properties = canlink_tscan_sys::MASK_CANPROP_DIR_TX;
    if is_ext {
        properties |= canlink_tscan_sys::MASK_CANPROP_EXTEND;
    }

    let mut fd_properties = canlink_tscan_sys::MASK_CANFDPROP_IS_FD;
    if brs {
        fd_properties |= canlink_tscan_sys::MASK_CANFDPROP_IS_BRS;
    }
    if esi {
        fd_properties |= canlink_tscan_sys::MASK_CANFDPROP_IS_ESI;
    }

    let mut payload = [0u8; 64];
    payload[..data.len()].copy_from_slice(data);

    Ok(TLIBCANFD {
        FIdxChn: channel,
        FProperties: properties,
        FDLC: len_to_dlc(data.len() as u8),
        FFDProperties: fd_properties,
        FIdentifier: can_id as i32,
        FTimeUs: 0,
        FData: payload,
    })
}

fn parse_can_frame(frame: &TLIBCAN) -> CanFrame {
    let id = frame.FIdentifier as u32;
    let is_ext = (frame.FProperties & canlink_tscan_sys::MASK_CANPROP_EXTEND) != 0;
    let data_len = usize::from(frame.FDLC.min(8));
    let data = frame.FData[..data_len].to_vec();
    CanFrame {
        id,
        is_ext,
        data,
        timestamp_us: if frame.FTimeUs > 0 {
            Some(frame.FTimeUs as u64)
        } else {
            None
        },
    }
}

fn parse_canfd_frame(frame: &TLIBCANFD) -> CanFdFrame {
    let id = frame.FIdentifier as u32;
    let is_ext = (frame.FProperties & canlink_tscan_sys::MASK_CANPROP_EXTEND) != 0;
    let data_len = dlc_to_len(frame.FDLC).min(64);
    let data = frame.FData[..data_len].to_vec();
    let brs = (frame.FFDProperties & canlink_tscan_sys::MASK_CANFDPROP_IS_BRS) != 0;
    let esi = (frame.FFDProperties & canlink_tscan_sys::MASK_CANFDPROP_IS_ESI) != 0;
    CanFdFrame {
        id,
        is_ext,
        brs,
        esi,
        data,
        timestamp_us: if frame.FTimeUs > 0 {
            Some(frame.FTimeUs as u64)
        } else {
            None
        },
    }
}

fn dlc_to_len(dlc: u8) -> usize {
    match dlc {
        0..=8 => dlc as usize,
        9 => 12,
        10 => 16,
        11 => 20,
        12 => 24,
        13 => 32,
        14 => 48,
        15 => 64,
        _ => 0,
    }
}

fn len_to_dlc(len: u8) -> u8 {
    match len {
        0..=8 => len,
        9..=12 => 9,
        13..=16 => 10,
        17..=20 => 11,
        21..=24 => 12,
        25..=32 => 13,
        33..=48 => 14,
        _ => 15,
    }
}
