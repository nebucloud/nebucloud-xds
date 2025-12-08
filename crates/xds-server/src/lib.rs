//! # xds-server
//!
//! xDS gRPC server implementation for control planes.
//!
//! This crate provides the gRPC server layer for xDS:
//!
//! - [`XdsServer`] - Main server type wrapping all xDS services
//! - [`XdsServerBuilder`] - Builder for configuring the server
//! - State-of-the-World (SotW) protocol support
//! - Delta xDS protocol support (incremental updates)
//!
//! ## Example
//!
//! ```rust,ignore
//! use xds_server::{XdsServer, XdsServerBuilder};
//! use xds_cache::ShardedCache;
//! use std::sync::Arc;
//!
//! let cache = Arc::new(ShardedCache::new());
//! let server = XdsServerBuilder::new()
//!     .cache(cache)
//!     .enable_sotw()
//!     .enable_delta()
//!     .build()?;
//!
//! // Use with tonic
//! tonic::transport::Server::builder()
//!     .add_service(server.ads_service())
//!     .serve("[::]:18000".parse()?).await?;
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(unsafe_code)]
#![warn(missing_docs)]

mod builder;
mod config;
mod delta;
mod sotw;
mod stream;

// Re-export service modules
pub mod services;

pub use builder::XdsServerBuilder;
pub use config::ServerConfig;
pub use stream::{StreamContext, StreamId};

use std::sync::Arc;
use xds_cache::ShardedCache;
use xds_core::ResourceRegistry;

/// The main xDS server.
///
/// This server wraps the cache and provides gRPC services for xDS.
#[derive(Debug)]
pub struct XdsServer {
    /// Shared cache.
    cache: Arc<ShardedCache>,
    /// Resource registry.
    registry: Arc<ResourceRegistry>,
    /// Server configuration.
    config: ServerConfig,
}

impl XdsServer {
    /// Create a new builder for configuring the server.
    pub fn builder() -> XdsServerBuilder {
        XdsServerBuilder::new()
    }

    /// Get a reference to the cache.
    #[inline]
    pub fn cache(&self) -> &Arc<ShardedCache> {
        &self.cache
    }

    /// Get a reference to the resource registry.
    #[inline]
    pub fn registry(&self) -> &Arc<ResourceRegistry> {
        &self.registry
    }

    /// Get the server configuration.
    #[inline]
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }
}
