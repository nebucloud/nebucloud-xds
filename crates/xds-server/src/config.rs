//! Server configuration.

use std::time::Duration;

/// Configuration for the xDS server.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Enable State-of-the-World protocol.
    pub enable_sotw: bool,
    /// Enable Delta xDS protocol.
    pub enable_delta: bool,
    /// Maximum concurrent streams per connection.
    pub max_concurrent_streams: Option<u32>,
    /// Keepalive interval.
    pub keepalive_interval: Option<Duration>,
    /// Keepalive timeout.
    pub keepalive_timeout: Option<Duration>,
    /// Maximum request size in bytes.
    pub max_request_size: usize,
    /// Response compression.
    pub compression: CompressionConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            enable_sotw: true,
            enable_delta: false,
            max_concurrent_streams: Some(100),
            keepalive_interval: Some(Duration::from_secs(30)),
            keepalive_timeout: Some(Duration::from_secs(10)),
            max_request_size: 4 * 1024 * 1024, // 4MB
            compression: CompressionConfig::default(),
        }
    }
}

/// Compression configuration.
#[derive(Debug, Clone, Default)]
pub struct CompressionConfig {
    /// Enable gzip compression for responses.
    pub gzip: bool,
    /// Minimum response size to compress (bytes).
    pub min_size: usize,
}

impl CompressionConfig {
    /// Create with gzip enabled.
    pub fn gzip() -> Self {
        Self {
            gzip: true,
            min_size: 1024,
        }
    }
}
