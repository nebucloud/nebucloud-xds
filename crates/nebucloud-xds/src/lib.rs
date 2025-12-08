//! # nebucloud-xds
//!
//! Production-ready xDS control plane library for Rust.
//!
//! This crate provides a complete implementation of the xDS protocol
//! for building Envoy control planes. It supports:
//!
//! - State-of-the-World (SotW) xDS protocol
//! - Delta xDS protocol (incremental updates)
//! - Aggregated Discovery Service (ADS)
//! - Individual xDS services (CDS, LDS, RDS, EDS, SDS)
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use nebucloud_xds::prelude::*;
//!
//! // Create a cache
//! let cache = ShardedCache::new();
//!
//! // Build a snapshot for a node
//! let snapshot = Snapshot::builder()
//!     .version("v1")
//!     .build();
//!
//! // Set snapshot
//! cache.set_snapshot(NodeHash::from_id("node-1"), snapshot);
//!
//! // Create server
//! let server = XdsServer::builder()
//!     .cache(Arc::new(cache))
//!     .enable_sotw()
//!     .build()?;
//! ```
//!
//! ## Architecture
//!
//! This library is organized into several crates:
//!
//! - `xds-core` - Core types, traits, and error handling
//! - `xds-cache` - Snapshot cache with watch notifications
//! - `xds-server` - gRPC server implementation
//! - `xds-types` - Generated protobuf types
//!
//! This crate (`nebucloud-xds`) re-exports all public APIs for convenience.
//!
//! ## Design Principles
//!
//! 1. **No panics in library code** - All errors are returned as `Result`
//! 2. **No locks held across await points** - Uses DashMap and careful design
//! 3. **Type-safe resources** - Generic `Resource` trait with registry
//! 4. **Observable** - Built-in metrics and tracing support
//!
//! ## Features
//!
//! - `full` - Enable all features (default)
//! - `sotw` - State-of-the-World protocol
//! - `delta` - Delta xDS protocol
//! - `compression` - Response compression

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(unsafe_code)]
#![warn(missing_docs)]

// Re-export all sub-crates
pub use xds_cache as cache;
pub use xds_core as core;
pub use xds_server as server;
pub use xds_types as types;

/// Prelude module for convenient imports.
///
/// ```rust,ignore
/// use nebucloud_xds::prelude::*;
/// ```
pub mod prelude {
    // Core types
    pub use xds_core::{
        BoxResource, NodeHash, Resource, ResourceRegistry, ResourceTypeInfo, ResourceVersion,
        SharedResourceRegistry, TypeUrl, XdsError, XdsResult,
    };

    // Cache types
    pub use xds_cache::{
        Cache, CacheStats, ShardedCache, Snapshot, SnapshotBuilder, Watch, WatchId,
    };

    // Server types
    pub use xds_server::{ServerConfig, StreamContext, StreamId, XdsServer, XdsServerBuilder};
}

/// Version information for this crate.
pub mod version {
    /// Crate version.
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");

    /// Minimum supported Rust version.
    pub const MSRV: &str = "1.75";

    /// Get version info as a string.
    pub fn version_string() -> String {
        format!("nebucloud-xds {} (MSRV {})", VERSION, MSRV)
    }
}

#[cfg(test)]
mod tests {
    use super::prelude::*;
    use std::sync::Arc;

    #[test]
    fn prelude_imports_work() {
        let cache = ShardedCache::new();
        let node = NodeHash::from_id("test-node");

        let snapshot = Snapshot::builder().version("v1").build();

        cache.set_snapshot(node, snapshot);

        let retrieved = cache.get_snapshot(node).unwrap();
        assert_eq!(retrieved.version(), "v1");
    }

    #[test]
    fn server_builder_works() {
        let cache = Arc::new(ShardedCache::new());

        let result = XdsServer::builder()
            .cache(cache)
            .enable_sotw()
            .enable_delta()
            .build();

        assert!(result.is_ok());
    }

    #[test]
    fn version_info() {
        let version = super::version::version_string();
        assert!(version.contains("nebucloud-xds"));
    }
}
