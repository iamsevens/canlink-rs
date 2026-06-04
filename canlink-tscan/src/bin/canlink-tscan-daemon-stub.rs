use canlink_tscan::daemon::{
    read_frame, write_frame, CapabilityResult, ConnectResult, DeviceInfo, ErrorCode, HelloAck, Op,
    Request, Response, ScanResult,
};
use std::env;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

fn main() -> io::Result<()> {
    let protocol_version = env::var("PROTOCOL_VERSION")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(1);
    let hang_on = env::var("HANG_ON_OP").ok();
    let exit_once_on = env::var("EXIT_ON_OP_ONCE").ok();
    let lib_error_once_on = env::var("LIB_ERROR_ON_OP_ONCE").ok();
    let lib_error_code = env::var("LIB_ERROR_CODE")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(2);
    let trace_path = env::var("TRACE_PATH").ok();

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();

    loop {
        let request: Request = match read_frame(&mut input) {
            Ok(req) => req,
            Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(err) => return Err(err),
        };

        let op_name = op_name(&request.op);
        append_trace(&trace_path, format!("OP:{op_name}"))?;
        if exit_once_on
            .as_deref()
            .is_some_and(|expected| expected == op_name)
            && !trace_contains_marker(&trace_path, &format!("EXITED:{op_name}"))
        {
            append_trace(&trace_path, format!("EXITED:{op_name}"))?;
            std::process::exit(2);
        }
        if hang_on.as_deref() == Some(op_name) {
            loop {
                thread::sleep(Duration::from_secs(1));
            }
        }
        if lib_error_once_on
            .as_deref()
            .is_some_and(|expected| expected == op_name)
            && !trace_contains_marker(&trace_path, &format!("LIB_ERRORED:{op_name}"))
        {
            append_trace(&trace_path, format!("LIB_ERRORED:{op_name}"))?;
            let message = if op_name == "CONNECT" {
                format!("tscan_connect failed: {lib_error_code} (Connect failed)")
            } else {
                format!("{op_name} failed: {lib_error_code}")
            };
            write_frame(
                &mut output,
                &Response::error(request.id, ErrorCode::LibTscanError as u32, message),
            )?;
            continue;
        }

        let response = match request.op {
            Op::Hello { .. } => {
                append_trace(&trace_path, "HELLO".to_string())?;
                Response::ok_data(
                    request.id,
                    &HelloAck {
                        protocol_version,
                        daemon_version: "stub".to_string(),
                    },
                )
            }
            Op::InitLib { .. } => Response::ok_empty(request.id),
            Op::Scan => Response::ok_data(
                request.id,
                &ScanResult {
                    devices: vec![DeviceInfo {
                        manufacturer: "STUB".to_string(),
                        product: "STUB-DEVICE".to_string(),
                        serial: "STUB123456".to_string(),
                        device_type: 0,
                    }],
                },
            ),
            Op::GetDeviceInfo { index } => Response::ok_data(
                request.id,
                &DeviceInfo {
                    manufacturer: "STUB".to_string(),
                    product: "STUB-DEVICE".to_string(),
                    serial: format!("STUB-{index}"),
                    device_type: 0,
                },
            ),
            Op::Connect { serial } => {
                let actual_serial = if serial.is_empty() {
                    "STUB123456".to_string()
                } else {
                    serial
                };
                Response::ok_data(
                    request.id,
                    &ConnectResult {
                        handle: 1,
                        channel_count: 2,
                        supports_canfd: true,
                        serial: actual_serial,
                    },
                )
            }
            Op::DisconnectByHandle { .. } => Response::ok_empty(request.id),
            Op::DisconnectAll => Response::ok_empty(request.id),
            Op::OpenChannel { .. } => Response::ok_empty(request.id),
            Op::CloseChannel { .. } => Response::ok_empty(request.id),
            Op::ConfigCanBaudrate { .. } => Response::ok_empty(request.id),
            Op::ConfigCanfdBaudrate { .. } => Response::ok_empty(request.id),
            Op::SendCan { .. } => Response::ok_empty(request.id),
            Op::SendCanfd { .. } => Response::ok_empty(request.id),
            Op::RecvCan { max_count, .. } => {
                let count = stub_recv_count("STUB_RECV_CAN_MESSAGES").min(max_count as usize);
                let messages = (0..count)
                    .map(|index| {
                        serde_json::json!({
                            "id": 0x100 + index as u32,
                            "is_ext": false,
                            "data": [index as u8],
                            "timestamp_us": null,
                        })
                    })
                    .collect::<Vec<_>>();
                Response::ok(request.id, serde_json::json!({ "messages": messages }))
            }
            Op::RecvCanfd { .. } => Response::ok(request.id, serde_json::json!({ "messages": [] })),
            Op::GetCapability { .. } => Response::ok_data(
                request.id,
                &CapabilityResult {
                    channel_count: 2,
                    supports_canfd: true,
                    max_bitrate_kbps: 1000,
                    supported_bitrates_kbps: vec![125, 250, 500, 1000],
                },
            ),
            Op::Finalize => {
                let response = Response::ok_empty(request.id);
                write_frame(&mut output, &response)?;
                return Ok(());
            }
        };

        write_frame(&mut output, &response)?;
    }
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

fn append_trace(path: &Option<String>, line: String) -> io::Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{line}")?;
    Ok(())
}

fn trace_contains_marker(path: &Option<String>, marker: &str) -> bool {
    let Some(path) = path else {
        return false;
    };
    std::fs::read_to_string(path)
        .map(|content| content.lines().any(|line| line.trim() == marker))
        .unwrap_or(false)
}

fn stub_recv_count(env_name: &str) -> usize {
    env::var(env_name)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(0)
}
