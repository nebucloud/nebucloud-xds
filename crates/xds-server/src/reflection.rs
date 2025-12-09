//! gRPC Server Reflection support for xDS services.
//!
//! This module provides gRPC reflection capabilities, allowing clients to
//! discover available services and their message types at runtime. This is
//! especially useful for debugging and tooling like `grpcurl`.
//!
//! # Example
//!
//! ```ignore
//! use xds_server::reflection::ReflectionService;
//!
//! // Create reflection service with encoded file descriptors
//! let reflection = ReflectionService::new(&[
//!     xds_types::FILE_DESCRIPTOR_SET,
//! ]);
//! ```

#[cfg(feature = "reflection")]
use tonic_reflection::server::{ServerReflection, ServerReflectionServer};

/// Configuration for the gRPC reflection service.
#[derive(Debug, Clone)]
pub struct ReflectionConfig {
    /// Whether reflection is enabled.
    pub enabled: bool,
    /// File descriptor sets to include in reflection.
    pub file_descriptor_sets: Vec<&'static [u8]>,
}

impl Default for ReflectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            file_descriptor_sets: Vec::new(),
        }
    }
}

impl ReflectionConfig {
    /// Create a new reflection configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable reflection.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Add a file descriptor set.
    pub fn add_file_descriptor_set(mut self, fds: &'static [u8]) -> Self {
        self.file_descriptor_sets.push(fds);
        self
    }
}

/// Service for gRPC server reflection.
///
/// This wraps tonic-reflection to provide service discovery capabilities.
#[cfg(feature = "reflection")]
pub struct ReflectionService {
    config: ReflectionConfig,
}

#[cfg(feature = "reflection")]
impl ReflectionService {
    /// Create a new reflection service with the given file descriptor sets.
    pub fn new(file_descriptor_sets: &[&'static [u8]]) -> Self {
        Self {
            config: ReflectionConfig {
                enabled: true,
                file_descriptor_sets: file_descriptor_sets.to_vec(),
            },
        }
    }

    /// Create a new reflection service with a configuration.
    pub fn with_config(config: ReflectionConfig) -> Self {
        Self { config }
    }

    /// Build the reflection server.
    ///
    /// Returns `None` if reflection is disabled or no file descriptors are provided.
    pub fn build_server(
        &self,
    ) -> Option<ServerReflectionServer<impl ServerReflection>> {
        if !self.config.enabled || self.config.file_descriptor_sets.is_empty() {
            return None;
        }

        let mut builder = tonic_reflection::server::Builder::configure();

        for fds in &self.config.file_descriptor_sets {
            builder = builder.register_encoded_file_descriptor_set(fds);
        }

        builder.build_v1().ok()
    }

    /// Build the reflection server (v1alpha version for older clients).
    ///
    /// Returns `None` if reflection is disabled or no file descriptors are provided.
    pub fn build_server_v1alpha(
        &self,
    ) -> Option<tonic_reflection::server::v1alpha::ServerReflectionServer<impl tonic_reflection::server::v1alpha::ServerReflection>> {
        if !self.config.enabled || self.config.file_descriptor_sets.is_empty() {
            return None;
        }

        let mut builder = tonic_reflection::server::Builder::configure();

        for fds in &self.config.file_descriptor_sets {
            builder = builder.register_encoded_file_descriptor_set(fds);
        }

        builder.build_v1alpha().ok()
    }
}

#[cfg(feature = "reflection")]
impl std::fmt::Debug for ReflectionService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReflectionService")
            .field("enabled", &self.config.enabled)
            .field(
                "file_descriptor_count",
                &self.config.file_descriptor_sets.len(),
            )
            .finish()
    }
}

/// Placeholder for when reflection feature is disabled.
#[cfg(not(feature = "reflection"))]
pub struct ReflectionService {
    _private: (),
}

#[cfg(not(feature = "reflection"))]
impl ReflectionService {
    /// Create a new (no-op) reflection service.
    pub fn new(_file_descriptor_sets: &[&'static [u8]]) -> Self {
        Self { _private: () }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reflection_config_defaults() {
        let config = ReflectionConfig::default();
        assert!(config.enabled);
        assert!(config.file_descriptor_sets.is_empty());
    }

    #[test]
    fn reflection_config_builder() {
        let config = ReflectionConfig::new()
            .enabled(false);
        assert!(!config.enabled);
    }

    #[cfg(feature = "reflection")]
    #[test]
    fn reflection_service_disabled_returns_none() {
        let service = ReflectionService::with_config(
            ReflectionConfig::new().enabled(false),
        );
        assert!(service.build_server().is_none());
    }

    #[cfg(feature = "reflection")]
    #[test]
    fn reflection_service_empty_fds_returns_none() {
        let service = ReflectionService::new(&[]);
        assert!(service.build_server().is_none());
    }
}
