//! OBD-II 诊断示例
//!
//! 本示例演示如何使用 CANLink 进行 OBD-II 车辆诊断通信。
//!
//! OBD-II 使用标准的 CAN 协议进行通信：
//! - 诊断请求 ID: 0x7DF (广播) 或 0x7E0-0x7E7 (特定 ECU)
//! - 诊断响应 ID: 0x7E8-0x7EF
//!
//! ## 运行示例
//!
//! ```bash
//! # 使用 Mock 后端
//! cargo run --example obd2_diagnostics
//!
//! # 使用真实硬件
//! cargo run --example obd2_diagnostics -- --backend tscan
//! ```

use canlink_hal::{BackendConfig, BackendRegistry, CanBackend, CanId, CanMessage};
use canlink_mock::{MockBackend, MockBackendFactory, MockConfig};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// OBD-II 服务类型
#[derive(Debug, Clone, Copy)]
enum Obd2Service {
    /// 显示当前数据
    ShowCurrentData = 0x01,
    /// 显示冻结帧数据
    ShowFreezeFrameData = 0x02,
    /// 显示诊断故障码
    ShowDtc = 0x03,
    /// 清除故障码
    ClearDtc = 0x04,
}

/// 常用的 OBD-II PID
#[derive(Debug, Clone, Copy)]
enum Obd2Pid {
    /// 支持的 PID (01-20)
    SupportedPids = 0x00,
    /// 发动机冷却液温度
    EngineCoolantTemp = 0x05,
    /// 发动机转速
    EngineRpm = 0x0C,
    /// 车速
    VehicleSpeed = 0x0D,
    /// 节气门位置
    ThrottlePosition = 0x11,
    /// 燃油系统状态
    FuelSystemStatus = 0x03,
}

/// OBD-II 请求构建器
struct Obd2Request {
    service: Obd2Service,
    pid: Option<Obd2Pid>,
}

impl Obd2Request {
    fn new(service: Obd2Service) -> Self {
        Self { service, pid: None }
    }

    fn with_pid(mut self, pid: Obd2Pid) -> Self {
        self.pid = Some(pid);
        self
    }

    fn build(&self) -> Result<CanMessage, Box<dyn std::error::Error>> {
        let mut data = vec![0x02, self.service as u8];

        if let Some(pid) = self.pid {
            data[0] = 0x02; // 数据长度
            data.push(pid as u8);
        } else {
            data[0] = 0x01; // 只有服务，没有 PID
        }

        // 填充到 8 字节
        while data.len() < 8 {
            data.push(0x00);
        }

        // 使用广播地址 0x7DF
        CanMessage::new_standard(0x7DF, &data).map_err(|e| e.into())
    }
}

/// OBD-II 响应解析器
struct Obd2Response {
    service: u8,
    pid: u8,
    data: Vec<u8>,
}

impl Obd2Response {
    fn parse(msg: &CanMessage) -> Option<Self> {
        let data = msg.data();

        // 检查最小长度
        if data.len() < 3 {
            return None;
        }

        // 检查是否是响应 (服务 + 0x40)
        let service = data[1];
        if service < 0x40 {
            return None;
        }

        Some(Self {
            service: service - 0x40,
            pid: data[2],
            data: data[3..].to_vec(),
        })
    }

    fn decode_rpm(&self) -> Option<f32> {
        if self.pid == Obd2Pid::EngineRpm as u8 && self.data.len() >= 2 {
            let rpm = ((self.data[0] as u16) << 8 | self.data[1] as u16) as f32 / 4.0;
            Some(rpm)
        } else {
            None
        }
    }

    fn decode_speed(&self) -> Option<u8> {
        if self.pid == Obd2Pid::VehicleSpeed as u8 && !self.data.is_empty() {
            Some(self.data[0])
        } else {
            None
        }
    }

    fn decode_coolant_temp(&self) -> Option<i16> {
        if self.pid == Obd2Pid::EngineCoolantTemp as u8 && !self.data.is_empty() {
            Some(self.data[0] as i16 - 40)
        } else {
            None
        }
    }

    fn decode_throttle(&self) -> Option<f32> {
        if self.pid == Obd2Pid::ThrottlePosition as u8 && !self.data.is_empty() {
            Some(self.data[0] as f32 * 100.0 / 255.0)
        } else {
            None
        }
    }
}

/// 设置 Mock 后端，模拟 OBD-II 响应
fn setup_mock_backend() -> MockBackend {
    // 创建模拟的 OBD-II 响应
    let responses = vec![
        // 发动机转速响应: 2000 RPM
        CanMessage::new_standard(0x7E8, &[0x04, 0x41, 0x0C, 0x1F, 0x40, 0x00, 0x00, 0x00])
            .unwrap(),
        // 车速响应: 60 km/h
        CanMessage::new_standard(0x7E8, &[0x03, 0x41, 0x0D, 0x3C, 0x00, 0x00, 0x00, 0x00])
            .unwrap(),
        // 冷却液温度响应: 90°C
        CanMessage::new_standard(0x7E8, &[0x03, 0x41, 0x05, 0x82, 0x00, 0x00, 0x00, 0x00])
            .unwrap(),
        // 节气门位置响应: 25%
        CanMessage::new_standard(0x7E8, &[0x03, 0x41, 0x11, 0x40, 0x00, 0x00, 0x00, 0x00])
            .unwrap(),
    ];

    let config = MockConfig::with_preset_messages(responses);
    MockBackend::with_config(config)
}

/// 发送 OBD-II 请求并接收响应
fn query_obd2(
    backend: &mut dyn CanBackend,
    request: Obd2Request,
    timeout_ms: u64,
) -> Result<Option<Obd2Response>, Box<dyn std::error::Error>> {
    // 发送请求
    let msg = request.build()?;
    backend.send_message(&msg)?;

    println!(
        "发送请求: 服务={:02X}, PID={:02X?}",
        request.service as u8,
        request.pid.map(|p| p as u8)
    );

    // 等待响应
    let start = std::time::Instant::now();
    while start.elapsed().as_millis() < timeout_ms as u128 {
        if let Some(response_msg) = backend.receive_message()? {
            // 检查是否是 OBD-II 响应 (0x7E8-0x7EF)
            if let CanId::Standard(id) = response_msg.id() {
                if (0x7E8..=0x7EF).contains(&id) {
                    if let Some(response) = Obd2Response::parse(&response_msg) {
                        return Ok(Some(response));
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(10));
    }

    Ok(None)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== OBD-II 诊断示例 ===\n");

    // 设置后端
    let registry = BackendRegistry::global();
    registry.register(Arc::new(MockBackendFactory::new()))?;

    let config = BackendConfig::new("mock");
    let mut backend = setup_mock_backend();

    // 初始化
    backend.initialize(&config)?;
    backend.open_channel(0)?;

    println!("后端已初始化\n");

    // 1. 查询发动机转速
    println!("--- 查询发动机转速 ---");
    let request = Obd2Request::new(Obd2Service::ShowCurrentData).with_pid(Obd2Pid::EngineRpm);

    if let Some(response) = query_obd2(&mut backend, request, 1000)? {
        if let Some(rpm) = response.decode_rpm() {
            println!("✓ 发动机转速: {:.0} RPM\n", rpm);
        }
    } else {
        println!("✗ 未收到响应\n");
    }

    // 2. 查询车速
    println!("--- 查询车速 ---");
    let request = Obd2Request::new(Obd2Service::ShowCurrentData).with_pid(Obd2Pid::VehicleSpeed);

    if let Some(response) = query_obd2(&mut backend, request, 1000)? {
        if let Some(speed) = response.decode_speed() {
            println!("✓ 车速: {} km/h\n", speed);
        }
    } else {
        println!("✗ 未收到响应\n");
    }

    // 3. 查询冷却液温度
    println!("--- 查询冷却液温度 ---");
    let request =
        Obd2Request::new(Obd2Service::ShowCurrentData).with_pid(Obd2Pid::EngineCoolantTemp);

    if let Some(response) = query_obd2(&mut backend, request, 1000)? {
        if let Some(temp) = response.decode_coolant_temp() {
            println!("✓ 冷却液温度: {}°C\n", temp);
        }
    } else {
        println!("✗ 未收到响应\n");
    }

    // 4. 查询节气门位置
    println!("--- 查询节气门位置 ---");
    let request =
        Obd2Request::new(Obd2Service::ShowCurrentData).with_pid(Obd2Pid::ThrottlePosition);

    if let Some(response) = query_obd2(&mut backend, request, 1000)? {
        if let Some(throttle) = response.decode_throttle() {
            println!("✓ 节气门位置: {:.1}%\n", throttle);
        }
    } else {
        println!("✗ 未收到响应\n");
    }

    // 清理
    backend.close_channel(0)?;
    backend.close()?;

    println!("=== 示例完成 ===");

    Ok(())
}
