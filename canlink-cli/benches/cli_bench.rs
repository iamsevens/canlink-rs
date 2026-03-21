//! CLI command response time benchmarks.
//!
//! This benchmark measures the response time of CLI commands
//! to ensure they meet performance requirements.
//!
//! ## Benchmark Scenarios
//!
//! - `canlink list` - List available backends
//! - `canlink info <backend>` - Query backend capabilities
//! - `canlink validate <config>` - Validate configuration file
//!
//! ## Notes
//!
//! These benchmarks test the internal command logic, not the full
//! CLI binary execution (which would include process startup overhead).

use canlink_hal::{BackendConfig, BackendRegistry, CanBackend};
use canlink_mock::{MockBackend, MockBackendFactory};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;

/// Setup: Register mock backend for testing
fn setup_registry() {
    let registry = BackendRegistry::global();
    // Only register if not already registered
    if !registry.is_registered("mock") {
        let _ = registry.register(Arc::new(MockBackendFactory::new()));
    }
}

/// Benchmark the `list` command logic.
///
/// This measures the time to list all registered backends.
fn bench_list_command(c: &mut Criterion) {
    setup_registry();

    let mut group = c.benchmark_group("cli_list");

    group.bench_function("list_backends", |b| {
        b.iter(|| {
            let registry = BackendRegistry::global();
            let backends = registry.list_backends();
            black_box(backends)
        });
    });

    group.finish();
}

/// Benchmark the `info` command logic.
///
/// This measures the time to query backend information and capabilities.
fn bench_info_command(c: &mut Criterion) {
    setup_registry();

    let mut group = c.benchmark_group("cli_info");

    group.bench_function("get_backend_info", |b| {
        b.iter(|| {
            let registry = BackendRegistry::global();
            let info = registry.get_backend_info(black_box("mock"));
            black_box(info)
        });
    });

    // Benchmark full info command flow: create backend + get capability
    group.bench_function("full_info_flow", |b| {
        b.iter(|| {
            let registry = BackendRegistry::global();
            let config = BackendConfig::new("mock");
            let mut backend = registry.create(black_box("mock"), &config).unwrap();
            backend.initialize(&config).unwrap();
            let capability = backend.get_capability().unwrap();
            black_box(capability)
        });
    });

    group.finish();
}

/// Benchmark the `validate` command logic.
///
/// This measures the time to parse and validate configuration.
fn bench_validate_command(c: &mut Criterion) {
    let mut group = c.benchmark_group("cli_validate");

    let config_content = r#"
[backend]
type = "mock"
device_index = 0
retry_count = 3
retry_interval_ms = 1000
"#;

    group.bench_function("parse_config", |b| {
        b.iter(|| {
            let config: toml::Value = toml::from_str(black_box(config_content)).unwrap();
            black_box(config)
        });
    });

    group.bench_function("validate_backend_config", |b| {
        b.iter(|| {
            let config = BackendConfig::new(black_box("mock"));
            black_box(config)
        });
    });

    group.finish();
}

/// Benchmark JSON output formatting for backend list.
///
/// This measures the overhead of JSON serialization for CLI output.
fn bench_json_output(c: &mut Criterion) {
    setup_registry();

    let mut group = c.benchmark_group("cli_json_output");

    // Benchmark backend list serialization
    let registry = BackendRegistry::global();
    let backends = registry.list_backends();

    group.bench_function("serialize_backend_list", |b| {
        b.iter(|| {
            let json = serde_json::to_string(black_box(&backends)).unwrap();
            black_box(json)
        });
    });

    group.bench_function("serialize_backend_list_pretty", |b| {
        b.iter(|| {
            let json = serde_json::to_string_pretty(black_box(&backends)).unwrap();
            black_box(json)
        });
    });

    group.finish();
}

/// Benchmark registry operations used by CLI.
fn bench_registry_operations(c: &mut Criterion) {
    setup_registry();

    let mut group = c.benchmark_group("cli_registry");

    group.bench_function("is_registered", |b| {
        b.iter(|| {
            let registry = BackendRegistry::global();
            let result = registry.is_registered(black_box("mock"));
            black_box(result)
        });
    });

    group.bench_function("create_backend", |b| {
        b.iter(|| {
            let registry = BackendRegistry::global();
            let config = BackendConfig::new("mock");
            let backend = registry.create(black_box("mock"), &config).unwrap();
            black_box(backend)
        });
    });

    group.finish();
}

/// Benchmark MockBackend direct operations for comparison.
fn bench_mock_backend_direct(c: &mut Criterion) {
    let mut group = c.benchmark_group("cli_mock_direct");

    group.bench_function("create_and_init", |b| {
        b.iter(|| {
            let mut backend = MockBackend::new();
            let config = BackendConfig::new("mock");
            backend.initialize(&config).unwrap();
            black_box(backend)
        });
    });

    group.bench_function("get_capability", |b| {
        let mut backend = MockBackend::new();
        let config = BackendConfig::new("mock");
        backend.initialize(&config).unwrap();

        b.iter(|| {
            let cap = backend.get_capability().unwrap();
            black_box(cap)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_list_command,
    bench_info_command,
    bench_validate_command,
    bench_json_output,
    bench_registry_operations,
    bench_mock_backend_direct,
);

criterion_main!(benches);
