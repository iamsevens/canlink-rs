//! 多线程并发通信示例
//!
//! 本示例演示如何在多线程环境中使用 CANLink 进行并发通信。
//!
//! ## 涵盖的主题
//!
//! - 线程安全的后端访问
//! - 生产者-消费者模式
//! - 消息队列和通道
//! - 并发发送和接收
//! - 性能监控和统计
//!
//! ## 运行示例
//!
//! ```bash
//! cargo run --example multi_threaded
//! ```

use canlink_hal::{BackendConfig, CanBackend, CanId, CanMessage};
use canlink_mock::{MockBackend, MockConfig};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// 消息统计
#[derive(Debug, Default, Clone)]
struct MessageStats {
    sent: usize,
    received: usize,
    errors: usize,
}

/// 线程安全的统计计数器
#[derive(Clone)]
struct StatsCounter {
    stats: Arc<Mutex<MessageStats>>,
}

impl StatsCounter {
    fn new() -> Self {
        Self {
            stats: Arc::new(Mutex::new(MessageStats::default())),
        }
    }

    fn increment_sent(&self) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.sent += 1;
        }
    }

    fn increment_received(&self) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.received += 1;
        }
    }

    fn increment_errors(&self) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.errors += 1;
        }
    }

    fn get(&self) -> MessageStats {
        self.stats.lock().unwrap().clone()
    }
}

/// 生产者线程 - 持续发送消息
fn producer_thread(
    backend: Arc<Mutex<Box<dyn CanBackend>>>,
    thread_id: usize,
    message_count: usize,
    stats: StatsCounter,
) {
    println!("生产者线程 {} 启动", thread_id);

    for i in 0..message_count {
        // 创建消息
        let id = 0x100 + (thread_id * 0x10) as u16;
        let data = vec![
            thread_id as u8,
            (i >> 8) as u8,
            (i & 0xFF) as u8,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
        ];

        let msg = match CanMessage::new_standard(id, &data) {
            Ok(m) => m,
            Err(_) => {
                stats.increment_errors();
                continue;
            }
        };

        // 发送消息
        if let Ok(mut backend) = backend.lock() {
            match backend.send_message(&msg) {
                Ok(_) => {
                    stats.increment_sent();
                    if i % 100 == 0 {
                        println!("  生产者 {}: 已发送 {} 条消息", thread_id, i + 1);
                    }
                }
                Err(e) => {
                    eprintln!("  生产者 {} 发送错误: {:?}", thread_id, e);
                    stats.increment_errors();
                }
            }
        }

        // 模拟一些处理时间
        thread::sleep(Duration::from_micros(100));
    }

    println!("生产者线程 {} 完成", thread_id);
}

/// 消费者线程 - 持续接收消息
fn consumer_thread(
    backend: Arc<Mutex<Box<dyn CanBackend>>>,
    thread_id: usize,
    duration: Duration,
    stats: StatsCounter,
) {
    println!("消费者线程 {} 启动", thread_id);

    let start = Instant::now();
    let mut last_report = Instant::now();
    let mut local_count = 0;

    while start.elapsed() < duration {
        if let Ok(mut backend) = backend.lock() {
            match backend.receive_message() {
                Ok(Some(msg)) => {
                    stats.increment_received();
                    local_count += 1;

                    // 每秒报告一次
                    if last_report.elapsed() >= Duration::from_secs(1) {
                        println!("  消费者 {}: 已接收 {} 条消息", thread_id, local_count);
                        last_report = Instant::now();
                    }
                }
                Ok(None) => {
                    // 没有消息，短暂休眠
                    drop(backend); // 释放锁
                    thread::sleep(Duration::from_millis(10));
                }
                Err(e) => {
                    eprintln!("  消费者 {} 接收错误: {:?}", thread_id, e);
                    stats.increment_errors();
                }
            }
        } else {
            thread::sleep(Duration::from_millis(10));
        }
    }

    println!("消费者线程 {} 完成 (接收 {} 条)", thread_id, local_count);
}

/// 监控线程 - 定期报告统计信息
fn monitor_thread(stats: StatsCounter, duration: Duration) {
    println!("监控线程启动\n");

    let start = Instant::now();
    let mut last_stats = MessageStats::default();

    while start.elapsed() < duration {
        thread::sleep(Duration::from_secs(2));

        let current_stats = stats.get();
        let elapsed = start.elapsed().as_secs_f64();

        let sent_rate = (current_stats.sent - last_stats.sent) as f64 / 2.0;
        let recv_rate = (current_stats.received - last_stats.received) as f64 / 2.0;

        println!("\n--- 统计信息 (运行时间: {:.1}s) ---", elapsed);
        println!("  已发送: {} 条 ({:.1} 条/秒)", current_stats.sent, sent_rate);
        println!(
            "  已接收: {} 条 ({:.1} 条/秒)",
            current_stats.received, recv_rate
        );
        println!("  错误: {} 次", current_stats.errors);

        last_stats = current_stats;
    }

    println!("\n监控线程完成");
}

/// 场景 1: 多个生产者，单个消费者
fn scenario_multiple_producers() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 场景 1: 多个生产者，单个消费者 ===\n");

    // 创建后端
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config)?;
    backend.open_channel(0)?;

    let backend: Arc<Mutex<Box<dyn CanBackend>>> = Arc::new(Mutex::new(Box::new(backend)));
    let stats = StatsCounter::new();

    // 启动 3 个生产者线程
    let mut handles = vec![];
    for i in 0..3 {
        let backend_clone = Arc::clone(&backend);
        let stats_clone = stats.clone();

        let handle = thread::spawn(move || {
            producer_thread(backend_clone, i, 500, stats_clone);
        });
        handles.push(handle);
    }

    // 启动 1 个消费者线程
    let backend_clone = Arc::clone(&backend);
    let stats_clone = stats.clone();
    let consumer_handle = thread::spawn(move || {
        consumer_thread(backend_clone, 0, Duration::from_secs(10), stats_clone);
    });

    // 启动监控线程
    let stats_clone = stats.clone();
    let monitor_handle = thread::spawn(move || {
        monitor_thread(stats_clone, Duration::from_secs(10));
    });

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }
    consumer_handle.join().unwrap();
    monitor_handle.join().unwrap();

    // 最终统计
    let final_stats = stats.get();
    println!("\n--- 最终统计 ---");
    println!("  总发送: {} 条", final_stats.sent);
    println!("  总接收: {} 条", final_stats.received);
    println!("  总错误: {} 次", final_stats.errors);

    Ok(())
}

/// 场景 2: 单个生产者，多个消费者
fn scenario_multiple_consumers() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n\n=== 场景 2: 单个生产者，多个消费者 ===\n");

    // 创建后端，预设一些消息
    let preset_messages: Vec<CanMessage> = (0..1000)
        .map(|i| {
            CanMessage::new_standard(
                0x200 + (i % 16) as u16,
                &[
                    (i >> 8) as u8,
                    (i & 0xFF) as u8,
                    0x00,
                    0x00,
                    0x00,
                    0x00,
                    0x00,
                    0x00,
                ],
            )
            .unwrap()
        })
        .collect();

    let mock_config = MockConfig::with_preset_messages(preset_messages);
    let mut backend = MockBackend::with_config(mock_config);
    let config = BackendConfig::new("mock");
    backend.initialize(&config)?;
    backend.open_channel(0)?;

    let backend: Arc<Mutex<Box<dyn CanBackend>>> = Arc::new(Mutex::new(Box::new(backend)));
    let stats = StatsCounter::new();

    // 启动 1 个生产者线程
    let backend_clone = Arc::clone(&backend);
    let stats_clone = stats.clone();
    let producer_handle = thread::spawn(move || {
        producer_thread(backend_clone, 0, 500, stats_clone);
    });

    // 启动 3 个消费者线程
    let mut handles = vec![];
    for i in 0..3 {
        let backend_clone = Arc::clone(&backend);
        let stats_clone = stats.clone();

        let handle = thread::spawn(move || {
            consumer_thread(backend_clone, i, Duration::from_secs(8), stats_clone);
        });
        handles.push(handle);
    }

    // 启动监控线程
    let stats_clone = stats.clone();
    let monitor_handle = thread::spawn(move || {
        monitor_thread(stats_clone, Duration::from_secs(8));
    });

    // 等待所有线程完成
    producer_handle.join().unwrap();
    for handle in handles {
        handle.join().unwrap();
    }
    monitor_handle.join().unwrap();

    // 最终统计
    let final_stats = stats.get();
    println!("\n--- 最终统计 ---");
    println!("  总发送: {} 条", final_stats.sent);
    println!("  总接收: {} 条", final_stats.received);
    println!("  总错误: {} 次", final_stats.errors);

    Ok(())
}

/// 场景 3: 多个生产者和多个消费者
fn scenario_multiple_both() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n\n=== 场景 3: 多个生产者和多个消费者 ===\n");

    // 创建后端
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config)?;
    backend.open_channel(0)?;

    let backend: Arc<Mutex<Box<dyn CanBackend>>> = Arc::new(Mutex::new(Box::new(backend)));
    let stats = StatsCounter::new();

    // 启动 2 个生产者线程
    let mut producer_handles = vec![];
    for i in 0..2 {
        let backend_clone = Arc::clone(&backend);
        let stats_clone = stats.clone();

        let handle = thread::spawn(move || {
            producer_thread(backend_clone, i, 300, stats_clone);
        });
        producer_handles.push(handle);
    }

    // 启动 2 个消费者线程
    let mut consumer_handles = vec![];
    for i in 0..2 {
        let backend_clone = Arc::clone(&backend);
        let stats_clone = stats.clone();

        let handle = thread::spawn(move || {
            consumer_thread(backend_clone, i, Duration::from_secs(6), stats_clone);
        });
        consumer_handles.push(handle);
    }

    // 启动监控线程
    let stats_clone = stats.clone();
    let monitor_handle = thread::spawn(move || {
        monitor_thread(stats_clone, Duration::from_secs(6));
    });

    // 等待所有线程完成
    for handle in producer_handles {
        handle.join().unwrap();
    }
    for handle in consumer_handles {
        handle.join().unwrap();
    }
    monitor_handle.join().unwrap();

    // 最终统计
    let final_stats = stats.get();
    println!("\n--- 最终统计 ---");
    println!("  总发送: {} 条", final_stats.sent);
    println!("  总接收: {} 条", final_stats.received);
    println!("  总错误: {} 次", final_stats.errors);

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 多线程并发通信示例 ===\n");

    // 运行三个场景
    scenario_multiple_producers()?;
    scenario_multiple_consumers()?;
    scenario_multiple_both()?;

    println!("\n=== 所有场景完成 ===");

    Ok(())
}
