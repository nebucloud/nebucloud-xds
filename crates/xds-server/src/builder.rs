//! Server builder for configuring and creating the xDS server.

use std::sync::Arc;
use std::time::Duration;

use xds_cache::ShardedCache;
use xds_core::{ResourceRegistry, XdsError, XdsResult};

use crate::config::{CompressionConfig, ServerConfig};
use crate::connections::{ConnectionLimits, ConnectionTracker};
use crate::metrics::XdsMetrics;
use crate::shutdown::ShutdownController;
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
///     .enable_health_check()
///     .enable_metrics()
///     .graceful_shutdown(Duration::from_secs(30))
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
    // Production features
    enable_health: bool,
    enable_metrics: bool,
    enable_connection_tracking: bool,
    grace_period: Option<Duration>,
    connection_limits: Option<ConnectionLimits>,
}

impl XdsServerBuilder {
    /// Create a new server builder.
    pub fn new() -> Self {
        Self {
            enable_sotw: true,    // Enable SotW by default
            enable_health: true,  // Enable health by default
            enable_metrics: true, // Enable metrics by default
            enable_connection_tracking: true, // Enable connection tracking by default
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

    // Production feature builders

    /// Enable gRPC health checking (enabled by default).
    pub fn enable_health_check(mut self) -> Self {
        self.enable_health = true;
        self
    }

    /// Disable gRPC health checking.
    pub fn disable_health_check(mut self) -> Self {
        self.enable_health = false;
        self
    }

    /// Enable Prometheus metrics (enabled by default).
    pub fn enable_metrics(mut self) -> Self {
        self.enable_metrics = true;
        self
    }

    /// Disable Prometheus metrics.
    pub fn disable_metrics(mut self) -> Self {
        self.enable_metrics = false;
        self
    }

    /// Enable connection tracking (enabled by default).
    pub fn enable_connection_tracking(mut self) -> Self {
        self.enable_connection_tracking = true;
        self
    }

    /// Disable connection tracking.
    pub fn disable_connection_tracking(mut self) -> Self {
        self.enable_connection_tracking = false;
        self
    }

    /// Set the graceful shutdown grace period.
    ///
    /// During shutdown, the server will:
    /// 1. Stop accepting new connections
    /// 2. Mark health as not serving
    /// 3. Wait for existing connections to drain (up to grace period)
    /// 4. Force close remaining connections
    pub fn graceful_shutdown(mut self, grace_period: Duration) -> Self {
        self.grace_period = Some(grace_period);
        self
    }

    /// Set connection limits.
    ///
    /// Controls maximum total connections and per-IP limits.
    pub fn connection_limits(mut self, limits: ConnectionLimits) -> Self {
        self.connection_limits = Some(limits);
        self
    }

    /// Set maximum total connections.
    pub fn max_connections(mut self, max: u64) -> Self {
        let limits = self.connection_limits.get_or_insert_with(ConnectionLimits::default);
        limits.max_connections = max;
        self
    }

    /// Set maximum connections per IP address.
    pub fn max_connections_per_ip(mut self, max: u64) -> Self {
        let limits = self.connection_limits.get_or_insert_with(ConnectionLimits::default);
        limits.max_per_ip = max;
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
            grace_period: self.grace_period.unwrap_or(Duration::from_secs(30)),
            enable_health: self.enable_health,
            enable_metrics: self.enable_metrics,
            enable_connection_tracking: self.enable_connection_tracking,
        };

        // Create optional production components
        let metrics = if self.enable_metrics {
            Some(XdsMetrics::new())
        } else {
            None
        };

        let connections = if self.enable_connection_tracking {
            let limits = self.connection_limits.unwrap_or_default();
            Some(ConnectionTracker::new(limits))
        } else {
            None
        };

        let shutdown = ShutdownController::new();

        Ok(XdsServer {
            cache,
            registry,
            config,
            metrics,
            shutdown,
            connections,
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
            .expect("server should build successfully");

        assert!(server.config().enable_sotw);
        assert!(server.config().enable_delta);
        assert_eq!(server.config().max_concurrent_streams, Some(200));
    }
}
