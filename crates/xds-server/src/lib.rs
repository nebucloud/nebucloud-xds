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
//! - Health checking via gRPC health protocol
//! - Prometheus metrics for observability
//! - Graceful shutdown with connection draining
//! - Connection tracking and limits
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
//!     .enable_health_check()
//!     .enable_metrics()
//!     .build()?;
//!
//! // Use with tonic - includes health and metrics
//! server.serve("[::]:18000".parse()?).await?;
//! ```
//!
//! ## Production Features
//!
//! ### Health Checking
//!
//! The server implements the gRPC health checking protocol:
//!
//! ```rust,ignore
//! let server = XdsServerBuilder::new()
//!     .cache(cache)
//!     .enable_health_check()
//!     .build()?;
//!
//! // Health status is managed automatically
//! // Available at grpc.health.v1.Health/Check
//! ```
//!
//! ### Metrics
//!
//! Prometheus metrics are exposed for monitoring:
//!
//! ```rust,ignore
//! let server = XdsServerBuilder::new()
//!     .cache(cache)
//!     .enable_metrics()
//!     .metrics_endpoint("/metrics")
//!     .build()?;
//! ```
//!
//! ### Graceful Shutdown
//!
//! The server supports graceful shutdown with connection draining:
//!
//! ```rust,ignore
//! use std::time::Duration;
//!
//! let server = XdsServerBuilder::new()
//!     .cache(cache)
//!     .graceful_shutdown(Duration::from_secs(30))
//!     .build()?;
//!
//! // Server will drain connections on SIGTERM/SIGINT
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(unsafe_code)]
#![warn(missing_docs)]

mod builder;
mod config;
pub mod connections;
mod delta;
pub mod health;
pub mod metrics;
pub mod reflection;
pub mod shutdown;
mod sotw;
mod stream;

#[cfg(test)]
mod protocol_tests;

// Re-export service modules
pub mod services;

pub use builder::XdsServerBuilder;
pub use config::ServerConfig;
pub use connections::{ConnectionGuard, ConnectionLimits, ConnectionTracker};
pub use health::{HealthConfig, HealthService};
pub use metrics::XdsMetrics;
#[cfg(feature = "reflection")]
pub use reflection::{ReflectionConfig, ReflectionService};
pub use shutdown::{ShutdownConfig, ShutdownController};
pub use stream::{StreamContext, StreamId};

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::sync::oneshot;
use tonic::transport::Server;
use tracing::info;
use xds_cache::ShardedCache;
use xds_core::ResourceRegistry;

use crate::health::HealthService as HealthSvc;
use crate::services::ServiceState;

/// The main xDS server.
///
/// This server wraps the cache and provides gRPC services for xDS.
/// It includes production-ready features like health checking,
/// metrics, graceful shutdown, and connection tracking.
#[derive(Debug)]
pub struct XdsServer {
    /// Shared cache.
    cache: Arc<ShardedCache>,
    /// Resource registry.
    registry: Arc<ResourceRegistry>,
    /// Server configuration.
    config: ServerConfig,
    /// Metrics collector.
    metrics: Option<XdsMetrics>,
    /// Shutdown controller.
    shutdown: ShutdownController,
    /// Connection tracker.
    connections: Option<ConnectionTracker>,
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

    /// Get the metrics instance, if enabled.
    #[inline]
    pub fn metrics(&self) -> Option<&XdsMetrics> {
        self.metrics.as_ref()
    }

    /// Get the shutdown controller.
    #[inline]
    pub fn shutdown_controller(&self) -> &ShutdownController {
        &self.shutdown
    }

    /// Get the connection tracker, if enabled.
    #[inline]
    pub fn connections(&self) -> Option<&ConnectionTracker> {
        self.connections.as_ref()
    }

    /// Create the service state for all discovery services.
    pub fn service_state(&self) -> ServiceState {
        ServiceState::new(
            Arc::clone(&self.cache),
            Arc::clone(&self.registry),
            self.config.clone(),
        )
    }

    /// Build the router with all xDS services configured.
    ///
    /// This is an internal helper to reduce duplication between serve methods.
    async fn build_router(&self) -> (tonic::transport::server::Router, Option<HealthSvc>) {
        let state = self.service_state();
        let (ads, cds, lds, rds, eds, sds) = state.create_services();

        // Build the server with configured options
        let mut builder = Server::builder();

        if let Some(interval) = self.config.keepalive_interval {
            builder = builder.http2_keepalive_interval(Some(interval));
        }
        if let Some(timeout) = self.config.keepalive_timeout {
            builder = builder.http2_keepalive_timeout(Some(timeout));
        }
        if let Some(max_streams) = self.config.max_concurrent_streams {
            builder = builder.concurrency_limit_per_connection(max_streams as usize);
        }

        // Add all xDS services
        let mut router = builder
            .add_service(ads.into_service())
            .add_service(cds.into_service())
            .add_service(lds.into_service())
            .add_service(rds.into_service())
            .add_service(eds.into_service())
            .add_service(sds.into_service());

        // Add health service if enabled
        let health = if self.config.enable_health {
            let (health, health_svc) = HealthSvc::new();
            router = router.add_service(health_svc);
            health.set_all_serving().await;
            Some(health)
        } else {
            None
        };

        (router, health)
    }

    /// Start the server and listen on the given address.
    ///
    /// This method will:
    /// 1. Set up health checking (if enabled)
    /// 2. Register all xDS services
    /// 3. Handle graceful shutdown on SIGTERM/SIGINT
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let server = XdsServerBuilder::new()
    ///     .cache(cache)
    ///     .build()?;
    ///
    /// server.serve("[::]:18000".parse()?).await?;
    /// ```
    pub async fn serve(self, addr: SocketAddr) -> Result<(), tonic::transport::Error> {
        info!(addr = %addr, "starting xDS server");

        let (router, health) = self.build_router().await;
        let grace_period = self.config.grace_period;

        let serve_future = router.serve_with_shutdown(addr, async move {
            // Wait for shutdown signal
            shutdown::wait_for_signal().await;

            // Mark services as not serving (for load balancer drain)
            if let Some(ref health) = health {
                health.set_all_not_serving().await;
            }

            // Wait for grace period
            info!(grace_period = ?grace_period, "draining connections");
            tokio::time::sleep(grace_period).await;
        });

        info!(addr = %addr, "xDS server listening");
        serve_future.await
    }

    /// Start the server with a custom shutdown signal.
    ///
    /// This allows you to control shutdown programmatically.
    pub async fn serve_with_shutdown(
        self,
        addr: SocketAddr,
        shutdown_rx: oneshot::Receiver<()>,
    ) -> Result<(), tonic::transport::Error> {
        info!(addr = %addr, "starting xDS server with custom shutdown");

        let (router, health) = self.build_router().await;
        let grace_period = self.config.grace_period;

        let serve_future = router.serve_with_shutdown(addr, async move {
            let _ = shutdown_rx.await;

            if let Some(ref health) = health {
                health.set_all_not_serving().await;
            }

            info!(grace_period = ?grace_period, "draining connections");
            tokio::time::sleep(grace_period).await;
        });

        info!(addr = %addr, "xDS server listening");
        serve_future.await
    }
}
