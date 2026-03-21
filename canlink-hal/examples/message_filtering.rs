//! 消息过滤和路由示例
//!
//! 本示例演示如何实现高级的消息过滤和路由功能。
//!
//! ## 涵盖的主题
//!
//! - ID 范围过滤
//! - 数据内容过滤
//! - 消息路由到不同处理器
//! - 优先级队列
//! - 消息转换和转发
//!
//! ## 运行示例
//!
//! ```bash
//! cargo run --example message_filtering
//! ```

#![allow(dead_code)]

use canlink_hal::{CanBackend, CanId, CanMessage};
use canlink_mock::{MockBackend, MockConfig};
use std::collections::HashMap;

/// 消息过滤器 trait
trait MessageFilter: Send {
    fn matches(&self, msg: &CanMessage) -> bool;
    fn name(&self) -> &str;
}

/// ID 范围过滤器
struct IdRangeFilter {
    name: String,
    start: u32,
    end: u32,
}

impl IdRangeFilter {
    fn new(name: &str, start: u32, end: u32) -> Self {
        Self {
            name: name.to_string(),
            start,
            end,
        }
    }
}

impl MessageFilter for IdRangeFilter {
    fn matches(&self, msg: &CanMessage) -> bool {
        let id_value = match msg.id() {
            CanId::Standard(id) => id as u32,
            CanId::Extended(id) => id,
        };
        id_value >= self.start && id_value <= self.end
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// 精确 ID 过滤器
struct ExactIdFilter {
    name: String,
    id: CanId,
}

impl ExactIdFilter {
    fn new(name: &str, id: CanId) -> Self {
        Self {
            name: name.to_string(),
            id,
        }
    }
}

impl MessageFilter for ExactIdFilter {
    fn matches(&self, msg: &CanMessage) -> bool {
        msg.id() == self.id
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// 数据内容过滤器
struct DataPatternFilter {
    name: String,
    pattern: Vec<Option<u8>>, // None 表示通配符
}

impl DataPatternFilter {
    fn new(name: &str, pattern: Vec<Option<u8>>) -> Self {
        Self {
            name: name.to_string(),
            pattern,
        }
    }
}

impl MessageFilter for DataPatternFilter {
    fn matches(&self, msg: &CanMessage) -> bool {
        let data = msg.data();

        if data.len() < self.pattern.len() {
            return false;
        }

        for (i, pattern_byte) in self.pattern.iter().enumerate() {
            if let Some(expected) = pattern_byte {
                if data[i] != *expected {
                    return false;
                }
            }
            // None 是通配符，匹配任何值
        }

        true
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// 组合过滤器 (AND 逻辑)
struct AndFilter {
    name: String,
    filters: Vec<Box<dyn MessageFilter>>,
}

impl AndFilter {
    fn new(name: &str, filters: Vec<Box<dyn MessageFilter>>) -> Self {
        Self {
            name: name.to_string(),
            filters,
        }
    }
}

impl MessageFilter for AndFilter {
    fn matches(&self, msg: &CanMessage) -> bool {
        self.filters.iter().all(|f| f.matches(msg))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// 消息处理器 trait
trait MessageHandler: Send {
    fn handle(&mut self, msg: &CanMessage);
    fn name(&self) -> &str;
}

/// 打印处理器
struct PrintHandler {
    name: String,
    count: usize,
}

impl PrintHandler {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            count: 0,
        }
    }
}

impl MessageHandler for PrintHandler {
    fn handle(&mut self, msg: &CanMessage) {
        self.count += 1;
        let id_value = match msg.id() {
            CanId::Standard(id) => id as u32,
            CanId::Extended(id) => id,
        };
        println!(
            "  [{}] 消息 #{}: ID=0x{:X}, 数据={:02X?}",
            self.name,
            self.count,
            id_value,
            msg.data()
        );
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// 统计处理器
struct StatsHandler {
    name: String,
    count: usize,
    id_counts: HashMap<u32, usize>,
}

impl StatsHandler {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            count: 0,
            id_counts: HashMap::new(),
        }
    }

    fn print_stats(&self) {
        println!("\n  [{}] 统计信息:", self.name);
        println!("    总消息数: {}", self.count);
        println!("    不同 ID 数: {}", self.id_counts.len());

        let mut sorted: Vec<_> = self.id_counts.iter().collect();
        sorted.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

        println!("    Top 5 ID:");
        for (id, count) in sorted.iter().take(5) {
            println!("      0x{:X}: {} 条", id, count);
        }
    }
}

impl MessageHandler for StatsHandler {
    fn handle(&mut self, msg: &CanMessage) {
        self.count += 1;

        let id_value = match msg.id() {
            CanId::Standard(id) => id as u32,
            CanId::Extended(id) => id,
        };

        *self.id_counts.entry(id_value).or_insert(0) += 1;
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// 消息路由器
struct MessageRouter {
    routes: Vec<(Box<dyn MessageFilter>, Box<dyn MessageHandler>)>,
}

impl MessageRouter {
    fn new() -> Self {
        Self { routes: Vec::new() }
    }

    fn add_route(&mut self, filter: Box<dyn MessageFilter>, handler: Box<dyn MessageHandler>) {
        self.routes.push((filter, handler));
    }

    fn route(&mut self, msg: &CanMessage) {
        for (filter, handler) in &mut self.routes {
            if filter.matches(msg) {
                handler.handle(msg);
            }
        }
    }

    fn print_all_stats(&self) {
        for (_, handler) in &self.routes {
            if let Some(stats) = (handler as &dyn std::any::Any).downcast_ref::<StatsHandler>() {
                stats.print_stats();
            }
        }
    }
}

/// 场景 1: 基础过滤
fn scenario_basic_filtering() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 场景 1: 基础消息过滤 ===\n");

    // 创建测试消息
    let messages = vec![
        CanMessage::new_standard(0x100, &[0x01, 0x02, 0x03])?,
        CanMessage::new_standard(0x150, &[0x04, 0x05, 0x06])?,
        CanMessage::new_standard(0x200, &[0x07, 0x08, 0x09])?,
        CanMessage::new_standard(0x250, &[0x0A, 0x0B, 0x0C])?,
        CanMessage::new_standard(0x300, &[0x0D, 0x0E, 0x0F])?,
    ];

    // 创建路由器
    let mut router = MessageRouter::new();

    // 路由 1: ID 0x100-0x1FF
    router.add_route(
        Box::new(IdRangeFilter::new("低地址", 0x100, 0x1FF)),
        Box::new(PrintHandler::new("低地址处理器")),
    );

    // 路由 2: ID 0x200-0x2FF
    router.add_route(
        Box::new(IdRangeFilter::new("中地址", 0x200, 0x2FF)),
        Box::new(PrintHandler::new("中地址处理器")),
    );

    // 路由 3: ID 0x300-0x3FF
    router.add_route(
        Box::new(IdRangeFilter::new("高地址", 0x300, 0x3FF)),
        Box::new(PrintHandler::new("高地址处理器")),
    );

    // 处理消息
    println!("处理 {} 条消息:\n", messages.len());
    for msg in &messages {
        router.route(msg);
    }

    Ok(())
}

/// 场景 2: 数据模式过滤
fn scenario_data_pattern_filtering() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n\n=== 场景 2: 数据模式过滤 ===\n");

    // 创建测试消息
    let messages = vec![
        CanMessage::new_standard(0x100, &[0x01, 0x02, 0x03, 0x04])?, // 匹配模式 1
        CanMessage::new_standard(0x101, &[0x01, 0xFF, 0x03, 0x04])?, // 匹配模式 1
        CanMessage::new_standard(0x102, &[0x05, 0x06, 0x07, 0x08])?, // 不匹配
        CanMessage::new_standard(0x103, &[0x01, 0x02, 0x09, 0x0A])?, // 匹配模式 1
        CanMessage::new_standard(0x104, &[0xAA, 0xBB, 0xCC, 0xDD])?, // 匹配模式 2
    ];

    // 创建路由器
    let mut router = MessageRouter::new();

    // 模式 1: 第一个字节是 0x01，第三个字节是 0x03 (第二个字节任意)
    router.add_route(
        Box::new(DataPatternFilter::new(
            "模式1",
            vec![Some(0x01), None, Some(0x03)],
        )),
        Box::new(PrintHandler::new("模式1处理器")),
    );

    // 模式 2: 第一个字节是 0xAA
    router.add_route(
        Box::new(DataPatternFilter::new("模式2", vec![Some(0xAA)])),
        Box::new(PrintHandler::new("模式2处理器")),
    );

    // 处理消息
    println!("处理 {} 条消息:\n", messages.len());
    for msg in &messages {
        router.route(msg);
    }

    Ok(())
}

/// 场景 3: 组合过滤和统计
fn scenario_combined_filtering() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n\n=== 场景 3: 组合过滤和统计 ===\n");

    // 创建后端，预设大量消息
    let mut preset_messages = Vec::new();
    for i in 0..1000 {
        let id = 0x100 + (i % 256) as u16;
        let data = vec![
            (i >> 8) as u8,
            (i & 0xFF) as u8,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
        ];
        preset_messages.push(CanMessage::new_standard(id, &data)?);
    }

    let mock_config = MockConfig::with_preset_messages(preset_messages);
    let mut backend = MockBackend::with_config(mock_config);
    let config = canlink_hal::BackendConfig::new("mock");
    backend.initialize(&config)?;
    backend.open_channel(0)?;

    // 创建路由器
    let mut router = MessageRouter::new();

    // 路由 1: 所有消息统计
    router.add_route(
        Box::new(IdRangeFilter::new("全部", 0x000, 0xFFF)),
        Box::new(StatsHandler::new("全局统计")),
    );

    // 路由 2: 低地址消息
    router.add_route(
        Box::new(IdRangeFilter::new("低地址", 0x100, 0x17F)),
        Box::new(StatsHandler::new("低地址统计")),
    );

    // 路由 3: 高地址消息
    router.add_route(
        Box::new(IdRangeFilter::new("高地址", 0x180, 0x1FF)),
        Box::new(StatsHandler::new("高地址统计")),
    );

    // 处理所有消息
    println!("处理消息中...\n");
    let mut count = 0;
    while let Some(msg) = backend.receive_message()? {
        router.route(&msg);
        count += 1;

        if count % 200 == 0 {
            println!("  已处理 {} 条消息", count);
        }
    }

    println!("\n总共处理 {} 条消息", count);

    // 打印统计
    router.print_all_stats();

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 消息过滤和路由示例 ===\n");

    scenario_basic_filtering()?;
    scenario_data_pattern_filtering()?;
    scenario_combined_filtering()?;

    println!("\n=== 所有场景完成 ===");

    Ok(())
}
