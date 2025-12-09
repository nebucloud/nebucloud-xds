//! Health service for gRPC health checking protocol.
//!
//! This module provides integration with `tonic-health` for the standard
//! gRPC health checking protocol (grpc.health.v1.Health).
//!
//! # Example
//!
//! ```rust,ignore
//! use xds_server::health::HealthService;
//!
//! let mut health = HealthService::new();
//! health.set_serving("xds").await;
//!
//! // Add to tonic server
//! Server::builder()
//!     .add_service(health.service())
//!     .serve(addr).await?;
//! ```

use std::sync::Arc;
use tokio::sync::Mutex;

use tonic_health::server::HealthReporter;

/// Health service wrapper for xDS server.
///
/// Provides gRPC health checking protocol support with service-level
/// health status management.
#[derive(Clone)]
pub struct HealthService {
    reporter: Arc<Mutex<HealthReporter>>,
}

impl std::fmt::Debug for HealthService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HealthService").finish()
    }
}

impl HealthService {
    /// Create a new health service and return both the wrapper and the tonic service.
    ///
    /// The returned tuple contains:
    /// - `HealthService` - wrapper for managing health status
    /// - The health server that can be added to a tonic router
    pub fn new() -> (Self, tonic_health::pb::health_server::HealthServer<impl tonic_health::pb::health_server::Health>) {
        let (reporter, service) = tonic_health::server::health_reporter();
        let wrapper = Self {
            reporter: Arc::new(Mutex::new(reporter)),
        };
        (wrapper, service)
    }

    /// Set a service as serving (healthy).
    pub async fn set_serving(&self, service_name: &str) {
        let mut reporter = self.reporter.lock().await;
        reporter
            .set_serving::<tonic_health::pb::health_server::HealthServer<
                tonic_health::server::HealthService,
            >>()
            .await;
        // Also set the specific service name
        reporter
            .set_service_status(service_name, tonic_health::ServingStatus::Serving)
            .await;
    }

    /// Set a service as not serving (unhealthy).
    pub async fn set_not_serving(&self, service_name: &str) {
        let mut reporter = self.reporter.lock().await;
        reporter
            .set_service_status(service_name, tonic_health::ServingStatus::NotServing)
            .await;
    }

    /// Set a service status to unknown.
    pub async fn set_unknown(&self, service_name: &str) {
        let mut reporter = self.reporter.lock().await;
        reporter
            .set_service_status(service_name, tonic_health::ServingStatus::Unknown)
            .await;
    }

    /// Set all xDS services as serving.
    pub async fn set_all_serving(&self) {
        self.set_all_status(tonic_health::ServingStatus::Serving).await;
    }

    /// Set all xDS services as not serving (for graceful shutdown).
    pub async fn set_all_not_serving(&self) {
        self.set_all_status(tonic_health::ServingStatus::NotServing).await;
    }

    /// Set status for all known xDS services.
    async fn set_all_status(&self, status: tonic_health::ServingStatus) {
        let mut reporter = self.reporter.lock().await;

        // Set the main server status
        if status == tonic_health::ServingStatus::Serving {
            reporter
                .set_serving::<tonic_health::pb::health_server::HealthServer<
                    tonic_health::server::HealthService,
                >>()
                .await;
        }

        // Set all xDS services to the requested status
        for service in Self::xds_service_names() {
            reporter.set_service_status(service, status).await;
        }
    }

    /// Get the list of xDS service names for health checking.
    ///
    /// This returns the standard Envoy xDS service names plus a generic "xds" entry.
    /// These names match the gRPC service names used by Envoy clients.
    #[inline]
    pub const fn xds_service_names() -> &'static [&'static str] {
        &[
            "envoy.service.discovery.v3.AggregatedDiscoveryService",
            "envoy.service.cluster.v3.ClusterDiscoveryService",
            "envoy.service.listener.v3.ListenerDiscoveryService",
            "envoy.service.route.v3.RouteDiscoveryService",
            "envoy.service.endpoint.v3.EndpointDiscoveryService",
            "envoy.service.secret.v3.SecretDiscoveryService",
            "xds",
        ]
    }
}

/// Health check configuration.
#[derive(Debug, Clone)]
pub struct HealthConfig {
    /// Enable health checking.
    pub enabled: bool,
    /// Initial status (serving or not).
    pub initial_serving: bool,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            initial_serving: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_service_creation() {
        let (health, _server) = HealthService::new();
        // Should not panic
        health.set_all_serving().await;
    }

    #[tokio::test]
    async fn health_service_status_transitions() {
        let (health, _server) = HealthService::new();

        health.set_serving("test-service").await;
        health.set_not_serving("test-service").await;
        health.set_unknown("test-service").await;
        // Should not panic
    }

    #[test]
    fn health_config_defaults() {
        let config = HealthConfig::default();
        assert!(config.enabled);
        assert!(config.initial_serving);
    }
}
