//! Server builder for configuring and creating the xDS server.

use std::sync::Arc;
use std::time::Duration;

use xds_cache::ShardedCache;
use xds_core::{ResourceRegistry, XdsError, XdsResult};

use crate::config::{CompressionConfig, ServerConfig};
use crate::XdsServer;

/// Builder for creating an [`XdsServer`].
///
/// # Example
///
/// ```rust,ignore
/// use xds_server::XdsServerBuilder;
/// use xds_cache::ShardedCache;
/// use std::sync::Arc;
///
/// let cache = Arc::new(ShardedCache::new());
/// let server = XdsServerBuilder::new()
///     .cache(cache)
///     .enable_sotw()
///     .enable_delta()
///     .max_concurrent_streams(200)
///     .build()?;
/// ```
#[derive(Debug, Default)]
pub struct XdsServerBuilder {
    cache: Option<Arc<ShardedCache>>,
    registry: Option<Arc<ResourceRegistry>>,
    enable_sotw: bool,
    enable_delta: bool,
    max_concurrent_streams: Option<u32>,
    keepalive_interval: Option<Duration>,
    keepalive_timeout: Option<Duration>,
    max_request_size: Option<usize>,
    compression: Option<CompressionConfig>,
}

impl XdsServerBuilder {
    /// Create a new server builder.
    pub fn new() -> Self {
        Self {
            enable_sotw: true, // Enable SotW by default
            ..Default::default()
        }
    }

    /// Set the cache to use.
    ///
    /// This is required.
    pub fn cache(mut self, cache: Arc<ShardedCache>) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Set the resource registry.
    ///
    /// If not set, a default registry will be created.
    pub fn registry(mut self, registry: Arc<ResourceRegistry>) -> Self {
        self.registry = Some(registry);
        self
    }

    /// Enable State-of-the-World protocol (enabled by default).
    pub fn enable_sotw(mut self) -> Self {
        self.enable_sotw = true;
        self
    }

    /// Disable State-of-the-World protocol.
    pub fn disable_sotw(mut self) -> Self {
        self.enable_sotw = false;
        self
    }

    /// Enable Delta xDS protocol.
    pub fn enable_delta(mut self) -> Self {
        self.enable_delta = true;
        self
    }

    /// Disable Delta xDS protocol.
    pub fn disable_delta(mut self) -> Self {
        self.enable_delta = false;
        self
    }

    /// Set maximum concurrent streams per connection.
    pub fn max_concurrent_streams(mut self, max: u32) -> Self {
        self.max_concurrent_streams = Some(max);
        self
    }

    /// Set keepalive interval.
    pub fn keepalive_interval(mut self, interval: Duration) -> Self {
        self.keepalive_interval = Some(interval);
        self
    }

    /// Set keepalive timeout.
    pub fn keepalive_timeout(mut self, timeout: Duration) -> Self {
        self.keepalive_timeout = Some(timeout);
        self
    }

    /// Set maximum request size in bytes.
    pub fn max_request_size(mut self, size: usize) -> Self {
        self.max_request_size = Some(size);
        self
    }

    /// Enable gzip compression for responses.
    pub fn enable_compression(mut self) -> Self {
        self.compression = Some(CompressionConfig::gzip());
        self
    }

    /// Set custom compression configuration.
    pub fn compression(mut self, config: CompressionConfig) -> Self {
        self.compression = Some(config);
        self
    }

    /// Build the server.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No cache was provided
    /// - Neither SotW nor Delta is enabled
    pub fn build(self) -> XdsResult<XdsServer> {
        let cache = self
            .cache
            .ok_or_else(|| XdsError::Configuration("cache is required".into()))?;

        if !self.enable_sotw && !self.enable_delta {
            return Err(XdsError::Configuration(
                "at least one protocol (SotW or Delta) must be enabled".into(),
            ));
        }

        let registry = self
            .registry
            .unwrap_or_else(|| Arc::new(ResourceRegistry::new()));

        let config = ServerConfig {
            enable_sotw: self.enable_sotw,
            enable_delta: self.enable_delta,
            max_concurrent_streams: self.max_concurrent_streams.or(Some(100)),
            keepalive_interval: self.keepalive_interval.or(Some(Duration::from_secs(30))),
            keepalive_timeout: self.keepalive_timeout.or(Some(Duration::from_secs(10))),
            max_request_size: self.max_request_size.unwrap_or(4 * 1024 * 1024),
            compression: self.compression.unwrap_or_default(),
        };

        Ok(XdsServer {
            cache,
            registry,
            config,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_requires_cache() {
        let result = XdsServerBuilder::new().build();
        assert!(result.is_err());
    }

    #[test]
    fn builder_requires_protocol() {
        let cache = Arc::new(ShardedCache::new());
        let result = XdsServerBuilder::new()
            .cache(cache)
            .disable_sotw()
            .disable_delta()
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn builder_success() {
        let cache = Arc::new(ShardedCache::new());
        let server = XdsServerBuilder::new()
            .cache(cache)
            .enable_sotw()
            .enable_delta()
            .max_concurrent_streams(200)
            .build()
            .unwrap();

        assert!(server.config().enable_sotw);
        assert!(server.config().enable_delta);
        assert_eq!(server.config().max_concurrent_streams, Some(200));
    }
}
