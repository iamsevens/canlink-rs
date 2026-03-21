use canlink_hal::{BackendConfig, CanBackend, CanMessage};
use canlink_mock::{MockBackend, MockConfig};
use std::future::Future;

pub fn create_initialized_backend() -> MockBackend {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();
    backend
}

#[allow(dead_code)]
pub fn create_backend_with_messages(messages: Vec<CanMessage>) -> MockBackend {
    let config = MockConfig::with_preset_messages(messages);
    let mut backend = MockBackend::with_config(config);
    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config).unwrap();
    backend.open_channel(0).unwrap();
    backend
}

pub fn run_local<F>(test_fn: F)
where
    F: Future<Output = ()>,
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let local = tokio::task::LocalSet::new();
    local.block_on(&rt, test_fn);
}
