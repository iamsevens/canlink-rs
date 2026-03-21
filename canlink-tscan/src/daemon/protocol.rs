use canlink_hal::{CanError, CanResult};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Request {
    pub id: u64,
    #[serde(flatten)]
    pub op: Op,
}

impl Request {
    #[must_use]
    pub fn new(id: u64, op: Op) -> Self {
        Self { id, op }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "op", content = "params")]
pub enum Op {
    #[serde(rename = "HELLO")]
    Hello {
        protocol_version: u32,
        client_version: String,
    },
    #[serde(rename = "INIT_LIB")]
    InitLib {
        enable_fifo: bool,
        enable_error_frame: bool,
        use_hw_time: bool,
    },
    #[serde(rename = "SCAN")]
    Scan,
    #[serde(rename = "GET_DEVICE_INFO")]
    GetDeviceInfo { index: u32 },
    #[serde(rename = "CONNECT")]
    Connect { serial: String },
    #[serde(rename = "DISCONNECT_BY_HANDLE")]
    DisconnectByHandle { handle: u64 },
    #[serde(rename = "DISCONNECT_ALL")]
    DisconnectAll,
    #[serde(rename = "OPEN_CHANNEL")]
    OpenChannel { handle: u64, channel: u8 },
    #[serde(rename = "CLOSE_CHANNEL")]
    CloseChannel { handle: u64, channel: u8 },
    #[serde(rename = "CONFIG_CAN_BAUDRATE")]
    ConfigCanBaudrate {
        handle: u64,
        channel: u8,
        rate_kbps: f64,
        term: u8,
    },
    #[serde(rename = "CONFIG_CANFD_BAUDRATE")]
    ConfigCanfdBaudrate {
        handle: u64,
        channel: u8,
        arb_kbps: f64,
        data_kbps: f64,
        ctrl_type: u8,
        ctrl_mode: u8,
        term: u8,
    },
    #[serde(rename = "SEND_CAN")]
    SendCan {
        handle: u64,
        channel: u8,
        id: u32,
        is_ext: bool,
        data: Vec<u8>,
    },
    #[serde(rename = "SEND_CANFD")]
    SendCanfd {
        handle: u64,
        channel: u8,
        id: u32,
        is_ext: bool,
        brs: bool,
        esi: bool,
        data: Vec<u8>,
    },
    #[serde(rename = "RECV_CAN")]
    RecvCan {
        handle: u64,
        channel: u8,
        max_count: u8,
        timeout_ms: u64,
    },
    #[serde(rename = "RECV_CANFD")]
    RecvCanfd {
        handle: u64,
        channel: u8,
        max_count: u8,
        timeout_ms: u64,
    },
    #[serde(rename = "GET_CAPABILITY")]
    GetCapability { handle: u64 },
    #[serde(rename = "FINALIZE")]
    Finalize,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct HelloAck {
    pub protocol_version: u32,
    pub daemon_version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    pub manufacturer: String,
    pub product: String,
    pub serial: String,
    pub device_type: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ScanResult {
    pub devices: Vec<DeviceInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ConnectResult {
    pub handle: u64,
    pub channel_count: u8,
    pub supports_canfd: bool,
    pub serial: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CanFrame {
    pub id: u32,
    pub is_ext: bool,
    pub data: Vec<u8>,
    pub timestamp_us: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CanFdFrame {
    pub id: u32,
    pub is_ext: bool,
    pub brs: bool,
    pub esi: bool,
    pub data: Vec<u8>,
    pub timestamp_us: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RecvCanResult {
    pub messages: Vec<CanFrame>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RecvCanfdResult {
    pub messages: Vec<CanFdFrame>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CapabilityResult {
    pub channel_count: u8,
    pub supports_canfd: bool,
    pub max_bitrate_kbps: u32,
    pub supported_bitrates_kbps: Vec<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ok,
    Error,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    Ok = 0,
    LibTscanError = 1,
    InvalidParams = 2,
    AlreadyConnected = 3,
    InvalidHandle = 4,
    InvalidChannel = 5,
    NoDevice = 6,
    InvalidIndex = 7,
    DeviceBusy = 8,
    ProtocolError = 9,
}

impl ErrorCode {
    #[must_use]
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Ok),
            1 => Some(Self::LibTscanError),
            2 => Some(Self::InvalidParams),
            3 => Some(Self::AlreadyConnected),
            4 => Some(Self::InvalidHandle),
            5 => Some(Self::InvalidChannel),
            6 => Some(Self::NoDevice),
            7 => Some(Self::InvalidIndex),
            8 => Some(Self::DeviceBusy),
            9 => Some(Self::ProtocolError),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Response {
    pub id: u64,
    pub status: Status,
    pub code: u32,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub data: serde_json::Value,
}

impl Response {
    #[must_use]
    pub fn ok(id: u64, data: serde_json::Value) -> Self {
        Self {
            id,
            status: Status::Ok,
            code: ErrorCode::Ok as u32,
            message: String::new(),
            data,
        }
    }

    #[must_use]
    pub fn ok_data<T: Serialize>(id: u64, data: &T) -> Self {
        match serde_json::to_value(data) {
            Ok(value) => Self::ok(id, value),
            Err(err) => Self::error(
                id,
                ErrorCode::ProtocolError as u32,
                format!("serialization failed: {err}"),
            ),
        }
    }

    #[must_use]
    pub fn ok_empty(id: u64) -> Self {
        Self::ok(id, serde_json::json!({}))
    }

    #[must_use]
    pub fn error(id: u64, code: u32, message: impl Into<String>) -> Self {
        Self {
            id,
            status: Status::Error,
            code,
            message: message.into(),
            data: serde_json::json!({}),
        }
    }

    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.status == Status::Ok
    }

    pub fn decode_data<T: DeserializeOwned>(&self) -> CanResult<T> {
        serde_json::from_value(self.data.clone()).map_err(|err| CanError::InvalidFormat {
            reason: format!("invalid response data: {err}"),
        })
    }
}

#[must_use]
pub fn is_idempotent_op(op: &Op) -> bool {
    matches!(
        op,
        Op::Scan | Op::GetDeviceInfo { .. } | Op::GetCapability { .. }
    )
}
