//! Resource trait and registry for xDS resources.
//!
//! This module provides the [`Resource`] trait for implementing custom
//! xDS resource types, and [`ResourceRegistry`] for managing resource types.

use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use dashmap::DashMap;

use crate::TypeUrl;

/// Trait for xDS resources.
///
/// Implement this trait to create custom xDS resource types that can be
/// stored in the cache and served via xDS.
///
/// # Example
///
/// ```rust
/// use xds_core::{Resource, TypeUrl};
/// use prost_types::Any;
/// use std::any::Any as StdAny;
///
/// #[derive(Debug)]
/// struct MyCluster {
///     name: String,
///     // ... other fields
/// }
///
/// impl Resource for MyCluster {
///     fn type_url(&self) -> &str {
///         TypeUrl::CLUSTER
///     }
///
///     fn name(&self) -> &str {
///         &self.name
///     }
///
///     fn encode(&self) -> Result<Any, Box<dyn std::error::Error + Send + Sync>> {
///         // Encode to protobuf Any
///         Ok(Any {
///             type_url: self.type_url().to_string(),
///             value: vec![], // actual encoding would go here
///         })
///     }
///
///     fn as_any(&self) -> &dyn StdAny {
///         self
///     }
/// }
/// ```
pub trait Resource: Send + Sync + fmt::Debug {
    /// Get the type URL for this resource.
    fn type_url(&self) -> &str;

    /// Get the resource name.
    fn name(&self) -> &str;

    /// Encode the resource to a protobuf Any message.
    fn encode(&self) -> Result<prost_types::Any, Box<dyn std::error::Error + Send + Sync>>;

    /// Get the resource version, if known.
    fn version(&self) -> Option<&str> {
        None
    }

    /// Convert to Any for downcasting.
    fn as_any(&self) -> &dyn Any;
}

/// Type alias for a boxed resource.
/// Uses Arc for efficient cloning and sharing across snapshots.
pub type BoxResource = Arc<dyn Resource>;

/// A wrapped Any message that implements Resource.
///
/// This allows storing raw protobuf Any messages as resources
/// without needing to decode them.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Public API surface, will be used by consumers
pub struct AnyResource {
    type_url: String,
    name: String,
    version: Option<String>,
    any: prost_types::Any,
}

#[allow(dead_code)] // Public API surface, will be used by consumers
impl AnyResource {
    /// Create a new AnyResource.
    #[must_use]
    pub fn new(
        type_url: impl Into<String>,
        name: impl Into<String>,
        any: prost_types::Any,
    ) -> Self {
        Self {
            type_url: type_url.into(),
            name: name.into(),
            version: None,
            any,
        }
    }

    /// Create a new AnyResource with a version.
    #[must_use]
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Get the inner Any message.
    #[must_use]
    pub fn inner(&self) -> &prost_types::Any {
        &self.any
    }

    /// Consume and return the inner Any message.
    #[must_use]
    pub fn into_inner(self) -> prost_types::Any {
        self.any
    }
}

impl Resource for AnyResource {
    fn type_url(&self) -> &str {
        &self.type_url
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn encode(&self) -> Result<prost_types::Any, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.any.clone())
    }

    fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Registry for resource types.
///
/// The registry maps type URLs to resource metadata and provides
/// utilities for working with different resource types.
#[derive(Debug, Default)]
pub struct ResourceRegistry {
    types: HashMap<String, ResourceTypeInfo>,
}

/// Information about a registered resource type.
#[derive(Debug, Clone)]
pub struct ResourceTypeInfo {
    /// The type URL.
    pub type_url: String,
    /// Short name for the type.
    pub short_name: String,
    /// Description of the resource type.
    pub description: String,
}

impl ResourceRegistry {
    /// Create a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a registry pre-populated with standard Envoy types.
    #[must_use]
    pub fn with_envoy_types() -> Self {
        let mut registry = Self::new();

        registry.register(ResourceTypeInfo {
            type_url: TypeUrl::CLUSTER.to_string(),
            short_name: "Cluster".to_string(),
            description: "Cluster Discovery Service (CDS)".to_string(),
        });

        registry.register(ResourceTypeInfo {
            type_url: TypeUrl::ENDPOINT.to_string(),
            short_name: "ClusterLoadAssignment".to_string(),
            description: "Endpoint Discovery Service (EDS)".to_string(),
        });

        registry.register(ResourceTypeInfo {
            type_url: TypeUrl::LISTENER.to_string(),
            short_name: "Listener".to_string(),
            description: "Listener Discovery Service (LDS)".to_string(),
        });

        registry.register(ResourceTypeInfo {
            type_url: TypeUrl::ROUTE.to_string(),
            short_name: "RouteConfiguration".to_string(),
            description: "Route Discovery Service (RDS)".to_string(),
        });

        registry.register(ResourceTypeInfo {
            type_url: TypeUrl::SECRET.to_string(),
            short_name: "Secret".to_string(),
            description: "Secret Discovery Service (SDS)".to_string(),
        });

        registry.register(ResourceTypeInfo {
            type_url: TypeUrl::RUNTIME.to_string(),
            short_name: "Runtime".to_string(),
            description: "Runtime Discovery Service (RTDS)".to_string(),
        });

        registry
    }

    /// Register a new resource type.
    pub fn register(&mut self, info: ResourceTypeInfo) {
        self.types.insert(info.type_url.clone(), info);
    }

    /// Get information about a resource type by type URL.
    #[must_use]
    pub fn get(&self, type_url: &str) -> Option<&ResourceTypeInfo> {
        self.types.get(type_url)
    }

    /// Check if a type URL is registered.
    #[must_use]
    pub fn contains(&self, type_url: &str) -> bool {
        self.types.contains_key(type_url)
    }

    /// Get all registered type URLs.
    #[must_use]
    pub fn type_urls(&self) -> Vec<&str> {
        self.types.keys().map(String::as_str).collect()
    }

    /// Get the number of registered types.
    #[must_use]
    pub fn len(&self) -> usize {
        self.types.len()
    }

    /// Check if the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    /// Iterate over all registered types.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &ResourceTypeInfo)> {
        self.types.iter()
    }
}

/// Thread-safe registry for resource types.
///
/// This is a concurrent version of [`ResourceRegistry`] that can be safely
/// shared across threads without external synchronization.
///
/// Uses `DashMap` internally for lock-free reads and efficient concurrent writes.
///
/// # Example
///
/// ```rust
/// use xds_core::{SharedResourceRegistry, TypeUrl};
/// use std::sync::Arc;
///
/// let registry = Arc::new(SharedResourceRegistry::with_envoy_types());
///
/// // Can be shared across threads safely
/// assert!(registry.contains(TypeUrl::CLUSTER));
/// assert!(registry.contains(TypeUrl::LISTENER));
/// ```
#[derive(Debug)]
pub struct SharedResourceRegistry {
    types: DashMap<String, ResourceTypeInfo>,
}

impl Default for SharedResourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedResourceRegistry {
    /// Create a new empty thread-safe registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            types: DashMap::new(),
        }
    }

    /// Create a thread-safe registry pre-populated with standard Envoy types.
    #[must_use]
    pub fn with_envoy_types() -> Self {
        let registry = Self::new();

        registry.register(ResourceTypeInfo {
            type_url: TypeUrl::CLUSTER.to_string(),
            short_name: "Cluster".to_string(),
            description: "Cluster Discovery Service (CDS)".to_string(),
        });

        registry.register(ResourceTypeInfo {
            type_url: TypeUrl::ENDPOINT.to_string(),
            short_name: "ClusterLoadAssignment".to_string(),
            description: "Endpoint Discovery Service (EDS)".to_string(),
        });

        registry.register(ResourceTypeInfo {
            type_url: TypeUrl::LISTENER.to_string(),
            short_name: "Listener".to_string(),
            description: "Listener Discovery Service (LDS)".to_string(),
        });

        registry.register(ResourceTypeInfo {
            type_url: TypeUrl::ROUTE.to_string(),
            short_name: "RouteConfiguration".to_string(),
            description: "Route Discovery Service (RDS)".to_string(),
        });

        registry.register(ResourceTypeInfo {
            type_url: TypeUrl::SECRET.to_string(),
            short_name: "Secret".to_string(),
            description: "Secret Discovery Service (SDS)".to_string(),
        });

        registry.register(ResourceTypeInfo {
            type_url: TypeUrl::RUNTIME.to_string(),
            short_name: "Runtime".to_string(),
            description: "Runtime Discovery Service (RTDS)".to_string(),
        });

        registry.register(ResourceTypeInfo {
            type_url: TypeUrl::SCOPED_ROUTE.to_string(),
            short_name: "ScopedRouteConfiguration".to_string(),
            description: "Scoped Route Discovery Service (SRDS)".to_string(),
        });

        registry.register(ResourceTypeInfo {
            type_url: TypeUrl::VIRTUAL_HOST.to_string(),
            short_name: "VirtualHost".to_string(),
            description: "Virtual Host Discovery Service (VHDS)".to_string(),
        });

        registry
    }

    /// Register a new resource type.
    ///
    /// This is safe to call from multiple threads concurrently.
    pub fn register(&self, info: ResourceTypeInfo) {
        self.types.insert(info.type_url.clone(), info);
    }

    /// Get information about a resource type by type URL.
    ///
    /// Returns `None` if the type is not registered.
    #[must_use]
    pub fn get(&self, type_url: &str) -> Option<ResourceTypeInfo> {
        self.types.get(type_url).map(|r| r.clone())
    }

    /// Check if a type URL is registered.
    #[must_use]
    pub fn contains(&self, type_url: &str) -> bool {
        self.types.contains_key(type_url)
    }

    /// Get all registered type URLs.
    #[must_use]
    pub fn type_urls(&self) -> Vec<String> {
        self.types.iter().map(|r| r.key().clone()).collect()
    }

    /// Get the number of registered types.
    #[must_use]
    pub fn len(&self) -> usize {
        self.types.len()
    }

    /// Check if the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    /// Validate that a type URL is registered.
    ///
    /// Returns `Ok(())` if registered, or an error with available types if not.
    pub fn validate(&self, type_url: &str) -> Result<(), String> {
        if self.contains(type_url) {
            Ok(())
        } else {
            let available: Vec<_> = self.type_urls();
            Err(format!(
                "Unknown resource type: {}. Available types: {:?}",
                type_url, available
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_any_resource() {
        let any = prost_types::Any {
            type_url: TypeUrl::CLUSTER.to_string(),
            value: vec![1, 2, 3],
        };

        let resource = AnyResource::new(TypeUrl::CLUSTER, "my-cluster", any);
        assert_eq!(resource.type_url(), TypeUrl::CLUSTER);
        assert_eq!(resource.name(), "my-cluster");
        assert!(resource.version().is_none());
    }

    #[test]
    fn test_any_resource_with_version() {
        let any = prost_types::Any {
            type_url: TypeUrl::CLUSTER.to_string(),
            value: vec![],
        };

        let resource = AnyResource::new(TypeUrl::CLUSTER, "my-cluster", any).with_version("v1");
        assert_eq!(resource.version(), Some("v1"));
    }

    #[test]
    fn test_registry_new() {
        let registry = ResourceRegistry::new();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_with_envoy_types() {
        let registry = ResourceRegistry::with_envoy_types();
        assert!(!registry.is_empty());
        assert!(registry.contains(TypeUrl::CLUSTER));
        assert!(registry.contains(TypeUrl::ENDPOINT));
    }

    #[test]
    fn test_registry_register() {
        let mut registry = ResourceRegistry::new();
        registry.register(ResourceTypeInfo {
            type_url: "custom.type".to_string(),
            short_name: "Custom".to_string(),
            description: "Custom type".to_string(),
        });

        assert!(registry.contains("custom.type"));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_registry_get() {
        let registry = ResourceRegistry::with_envoy_types();
        let info = registry.get(TypeUrl::CLUSTER);
        assert!(info.is_some(), "CLUSTER type should be registered");
        // Safe because we just asserted it's Some
        #[allow(clippy::unwrap_used)]
        let info = info.unwrap();
        assert_eq!(info.short_name, "Cluster");
    }

    // SharedResourceRegistry tests

    #[test]
    fn test_shared_registry_new() {
        let registry = SharedResourceRegistry::new();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_shared_registry_with_envoy_types() {
        let registry = SharedResourceRegistry::with_envoy_types();
        assert!(!registry.is_empty());
        assert!(registry.contains(TypeUrl::CLUSTER));
        assert!(registry.contains(TypeUrl::ENDPOINT));
        assert!(registry.contains(TypeUrl::LISTENER));
        assert!(registry.contains(TypeUrl::ROUTE));
        assert!(registry.contains(TypeUrl::SECRET));
        assert!(registry.contains(TypeUrl::RUNTIME));
        assert!(registry.contains(TypeUrl::SCOPED_ROUTE));
        assert!(registry.contains(TypeUrl::VIRTUAL_HOST));
        assert_eq!(registry.len(), 8);
    }

    #[test]
    fn test_shared_registry_register() {
        let registry = SharedResourceRegistry::new();
        registry.register(ResourceTypeInfo {
            type_url: "custom.type".to_string(),
            short_name: "Custom".to_string(),
            description: "Custom type".to_string(),
        });

        assert!(registry.contains("custom.type"));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_shared_registry_get() {
        let registry = SharedResourceRegistry::with_envoy_types();
        let info = registry.get(TypeUrl::CLUSTER);
        assert!(info.is_some(), "CLUSTER type should be registered");
        #[allow(clippy::unwrap_used)]
        let info = info.unwrap();
        assert_eq!(info.short_name, "Cluster");
    }

    #[test]
    fn test_shared_registry_validate() {
        let registry = SharedResourceRegistry::with_envoy_types();
        
        // Valid type should pass
        assert!(registry.validate(TypeUrl::CLUSTER).is_ok());
        
        // Invalid type should fail with helpful message
        let err = registry.validate("unknown.type").unwrap_err();
        assert!(err.contains("Unknown resource type"));
        assert!(err.contains("unknown.type"));
    }

    #[test]
    fn test_shared_registry_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let registry = Arc::new(SharedResourceRegistry::new());
        let mut handles = vec![];

        // Spawn multiple threads that register types concurrently
        for i in 0..10 {
            let reg = Arc::clone(&registry);
            handles.push(thread::spawn(move || {
                reg.register(ResourceTypeInfo {
                    type_url: format!("type.{}", i),
                    short_name: format!("Type{}", i),
                    description: format!("Type {} description", i),
                });
            }));
        }

        for handle in handles {
            handle.join().expect("Thread panicked");
        }

        assert_eq!(registry.len(), 10);
    }
}
