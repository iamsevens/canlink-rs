//! Backend registry for managing and discovering hardware backends.
//!
//! This module provides the `BackendRegistry` for registering, querying, and creating
//! backend instances at runtime.

use crate::{BackendConfig, BackendFactory, BackendVersion, CanBackend, CanError, CanResult};
use indexmap::IndexMap;
use std::sync::{Arc, OnceLock, RwLock};

/// Backend information.
///
/// Contains metadata about a registered backend.
///
/// # Examples
///
/// ```
/// use canlink_hal::BackendInfo;
///
/// let info = BackendInfo {
///     name: "mock".to_string(),
///     version: canlink_hal::BackendVersion::new(0, 1, 0),
/// };
/// println!("Backend: {} v{}", info.name, info.version);
/// ```
#[derive(Debug, Clone)]
pub struct BackendInfo {
    /// Backend name
    pub name: String,

    /// Backend version
    pub version: BackendVersion,
}

/// Backend registry.
///
/// Manages all registered hardware backends and provides methods for registration,
/// querying, and creating backend instances.
///
/// # Thread Safety
///
/// All methods of `BackendRegistry` are thread-safe and can be called from multiple
/// threads simultaneously. Internal state is protected by `RwLock`.
///
/// # Examples
///
/// ```rust,ignore
/// use canlink_hal::BackendRegistry;
///
/// // Register a backend
/// let registry = BackendRegistry::new();
/// registry.register(Box::new(MockBackendFactory::new()))?;
///
/// // Query available backends
/// let backends = registry.list_backends();
/// println!("Available backends: {:?}", backends);
///
/// // Create a backend instance
/// let backend = registry.create("mock", &config)?;
/// ```
pub struct BackendRegistry {
    factories: RwLock<IndexMap<String, Arc<dyn BackendFactory>>>,
}

impl BackendRegistry {
    /// Create a new backend registry.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::BackendRegistry;
    ///
    /// let registry = BackendRegistry::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            factories: RwLock::new(IndexMap::new()),
        }
    }

    /// Get the global registry instance (singleton).
    ///
    /// This returns a shared reference to the global registry. The global registry
    /// is initialized on first access and persists for the lifetime of the program.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::BackendRegistry;
    ///
    /// let registry = BackendRegistry::global();
    /// ```
    pub fn global() -> Arc<Self> {
        static INSTANCE: OnceLock<Arc<BackendRegistry>> = OnceLock::new();
        INSTANCE.get_or_init(|| Arc::new(Self::new())).clone()
    }

    /// Register a backend factory.
    ///
    /// Registers a new backend factory with the registry. The backend name must be unique.
    ///
    /// # Arguments
    ///
    /// * `factory` - Backend factory instance
    ///
    /// # Errors
    ///
    /// * `CanError::BackendAlreadyRegistered` - Backend name already registered
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe and can be called from multiple threads.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// registry.register(Box::new(MockBackendFactory::new()))?;
    /// ```
    pub fn register(&self, factory: Arc<dyn BackendFactory>) -> CanResult<()> {
        let name = factory.name().to_string();
        let mut factories = self.factories.write().map_err(|e| CanError::Other {
            message: format!("Failed to acquire write lock: {e}"),
        })?;

        if factories.contains_key(&name) {
            return Err(CanError::BackendAlreadyRegistered { name: name.clone() });
        }

        factories.insert(name, factory);
        Ok(())
    }

    /// Unregister a backend.
    ///
    /// Removes a backend factory from the registry.
    ///
    /// # Arguments
    ///
    /// * `name` - Backend name
    ///
    /// # Errors
    ///
    /// * `CanError::BackendNotFound` - Backend not registered
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// registry.unregister("mock")?;
    /// ```
    pub fn unregister(&self, name: &str) -> CanResult<()> {
        let mut factories = self.factories.write().map_err(|e| CanError::Other {
            message: format!("Failed to acquire write lock: {e}"),
        })?;

        if factories.shift_remove(name).is_none() {
            return Err(CanError::BackendNotFound {
                name: name.to_string(),
            });
        }

        Ok(())
    }

    /// Create a backend instance.
    ///
    /// Creates a new backend instance using the registered factory.
    ///
    /// # Arguments
    ///
    /// * `name` - Backend name
    /// * `config` - Backend configuration
    ///
    /// # Returns
    ///
    /// A boxed backend instance ready for initialization.
    ///
    /// # Errors
    ///
    /// * `CanError::BackendNotFound` - Backend not registered
    /// * `CanError::ConfigError` - Invalid configuration
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe and can be called from multiple threads.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let backend = registry.create("mock", &config)?;
    /// ```
    pub fn create(&self, name: &str, config: &BackendConfig) -> CanResult<Box<dyn CanBackend>> {
        let factories = self.factories.read().map_err(|e| CanError::Other {
            message: format!("Failed to acquire read lock: {e}"),
        })?;

        let factory = factories
            .get(name)
            .ok_or_else(|| CanError::BackendNotFound {
                name: name.to_string(),
            })?;

        factory.create(config)
    }

    /// List all registered backends.
    ///
    /// Returns a list of all backend names currently registered, in registration order.
    /// Backends are returned in the order they were registered, with earlier registrations
    /// appearing first in the list.
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned (should never happen in normal operation).
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let backends = registry.list_backends();
    /// for name in backends {
    ///     println!("Available: {}", name);
    /// }
    /// ```
    pub fn list_backends(&self) -> Vec<String> {
        let factories = self.factories.read().unwrap();
        // IndexMap preserves insertion order
        factories.keys().cloned().collect()
    }

    /// Get backend information.
    ///
    /// Returns metadata about a registered backend.
    ///
    /// # Arguments
    ///
    /// * `name` - Backend name
    ///
    /// # Errors
    ///
    /// * `CanError::BackendNotFound` - Backend not registered
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let info = registry.get_backend_info("mock")?;
    /// println!("Backend: {} v{}", info.name, info.version);
    /// ```
    pub fn get_backend_info(&self, name: &str) -> CanResult<BackendInfo> {
        let factories = self.factories.read().map_err(|e| CanError::Other {
            message: format!("Failed to acquire read lock: {e}"),
        })?;

        let factory = factories
            .get(name)
            .ok_or_else(|| CanError::BackendNotFound {
                name: name.to_string(),
            })?;

        Ok(BackendInfo {
            name: factory.name().to_string(),
            version: factory.version(),
        })
    }

    /// Check if a backend is registered.
    ///
    /// # Arguments
    ///
    /// * `name` - Backend name
    ///
    /// # Returns
    ///
    /// `true` if the backend is registered, `false` otherwise.
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned (should never happen in normal operation).
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// if registry.is_registered("mock") {
    ///     println!("Mock backend is available");
    /// }
    /// ```
    #[must_use]
    pub fn is_registered(&self, name: &str) -> bool {
        let factories = self.factories.read().unwrap();
        factories.contains_key(name)
    }
}

impl Default for BackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CanMessage, HardwareCapability};

    // Mock backend for testing
    struct MockBackend;

    impl CanBackend for MockBackend {
        fn initialize(&mut self, _config: &BackendConfig) -> CanResult<()> {
            Ok(())
        }

        fn close(&mut self) -> CanResult<()> {
            Ok(())
        }

        fn get_capability(&self) -> CanResult<HardwareCapability> {
            Ok(HardwareCapability::new(
                2,
                true,
                8_000_000,
                vec![125_000, 250_000, 500_000, 1_000_000],
                16,
                crate::TimestampPrecision::Microsecond,
            ))
        }

        fn send_message(&mut self, _message: &CanMessage) -> CanResult<()> {
            Ok(())
        }

        fn receive_message(&mut self) -> CanResult<Option<CanMessage>> {
            Ok(None)
        }

        fn open_channel(&mut self, _channel: u8) -> CanResult<()> {
            Ok(())
        }

        fn close_channel(&mut self, _channel: u8) -> CanResult<()> {
            Ok(())
        }

        fn version(&self) -> BackendVersion {
            BackendVersion::new(0, 1, 0)
        }

        fn name(&self) -> &'static str {
            "mock"
        }
    }

    struct MockBackendFactory;

    impl BackendFactory for MockBackendFactory {
        fn create(&self, _config: &BackendConfig) -> CanResult<Box<dyn CanBackend>> {
            Ok(Box::new(MockBackend))
        }

        fn name(&self) -> &'static str {
            "mock"
        }

        fn version(&self) -> BackendVersion {
            BackendVersion::new(0, 1, 0)
        }
    }

    #[test]
    fn test_registry_new() {
        let registry = BackendRegistry::new();
        assert_eq!(registry.list_backends().len(), 0);
    }

    #[test]
    fn test_registry_register() {
        let registry = BackendRegistry::new();
        let factory = Arc::new(MockBackendFactory);

        assert!(registry.register(factory).is_ok());
        assert_eq!(registry.list_backends().len(), 1);
        assert!(registry.is_registered("mock"));
    }

    #[test]
    fn test_registry_register_duplicate() {
        let registry = BackendRegistry::new();
        let factory1 = Arc::new(MockBackendFactory);
        let factory2 = Arc::new(MockBackendFactory);

        assert!(registry.register(factory1).is_ok());
        assert!(registry.register(factory2).is_err());
    }

    #[test]
    fn test_registry_unregister() {
        let registry = BackendRegistry::new();
        let factory = Arc::new(MockBackendFactory);

        registry.register(factory).unwrap();
        assert!(registry.is_registered("mock"));

        assert!(registry.unregister("mock").is_ok());
        assert!(!registry.is_registered("mock"));
    }

    #[test]
    fn test_registry_unregister_not_found() {
        let registry = BackendRegistry::new();
        assert!(registry.unregister("nonexistent").is_err());
    }

    #[test]
    fn test_registry_create() {
        let registry = BackendRegistry::new();
        let factory = Arc::new(MockBackendFactory);
        let config = BackendConfig::new("mock");

        registry.register(factory).unwrap();

        let backend = registry.create("mock", &config);
        assert!(backend.is_ok());
    }

    #[test]
    fn test_registry_create_not_found() {
        let registry = BackendRegistry::new();
        let config = BackendConfig::new("mock");

        let result = registry.create("nonexistent", &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_list_backends() {
        let registry = BackendRegistry::new();
        let factory = Arc::new(MockBackendFactory);

        registry.register(factory).unwrap();

        let backends = registry.list_backends();
        assert_eq!(backends.len(), 1);
        assert!(backends.contains(&"mock".to_string()));
    }

    #[test]
    fn test_registry_get_backend_info() {
        let registry = BackendRegistry::new();
        let factory = Arc::new(MockBackendFactory);

        registry.register(factory).unwrap();

        let info = registry.get_backend_info("mock").unwrap();
        assert_eq!(info.name, "mock");
        assert_eq!(info.version.major(), 0);
        assert_eq!(info.version.minor(), 1);
        assert_eq!(info.version.patch(), 0);
    }

    #[test]
    fn test_registry_global() {
        let registry1 = BackendRegistry::global();
        let registry2 = BackendRegistry::global();

        // Should be the same instance
        assert!(Arc::ptr_eq(&registry1, &registry2));
    }

    #[test]
    fn test_registration_order() {
        // Create test factories with different names
        struct TestFactory(&'static str);
        impl BackendFactory for TestFactory {
            fn create(&self, _config: &BackendConfig) -> CanResult<Box<dyn CanBackend>> {
                Ok(Box::new(MockBackend))
            }
            fn name(&self) -> &str {
                self.0
            }
            fn version(&self) -> BackendVersion {
                BackendVersion::new(0, 1, 0)
            }
        }

        let registry = BackendRegistry::new();

        // Register in specific order
        registry
            .register(Arc::new(TestFactory("backend_a")))
            .unwrap();
        registry
            .register(Arc::new(TestFactory("backend_b")))
            .unwrap();
        registry
            .register(Arc::new(TestFactory("backend_c")))
            .unwrap();

        // Verify order is preserved
        let backends = registry.list_backends();
        assert_eq!(backends, vec!["backend_a", "backend_b", "backend_c"]);
    }

    #[test]
    fn test_duplicate_registration_error_type() {
        let registry = BackendRegistry::new();
        let factory1 = Arc::new(MockBackendFactory);
        let factory2 = Arc::new(MockBackendFactory);

        registry.register(factory1).unwrap();

        let result = registry.register(factory2);
        assert!(result.is_err());

        // Verify error type
        match result.unwrap_err() {
            CanError::BackendAlreadyRegistered { name } => {
                assert_eq!(name, "mock");
            }
            other => panic!("Expected BackendAlreadyRegistered, got {other:?}"),
        }
    }
}
