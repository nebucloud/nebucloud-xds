//! gRPC service implementations for xDS.
//!
//! This module contains the tonic service implementations for:
//! - Aggregated Discovery Service (ADS)
//! - Cluster Discovery Service (CDS)
//! - Listener Discovery Service (LDS)
//! - Route Discovery Service (RDS)
//! - Endpoint Discovery Service (EDS)
//! - Secret Discovery Service (SDS)
//!
//! Note: These services require the `xds-types` crate for protobuf types.

// Placeholder for gRPC service implementations
// These will be implemented when xds-types provides the generated protobuf types

use std::sync::Arc;

use xds_cache::ShardedCache;
use xds_core::ResourceRegistry;

use crate::config::ServerConfig;
use crate::delta::DeltaHandler;
use crate::sotw::SotwHandler;

/// Shared state for all xDS services.
#[derive(Debug, Clone)]
pub struct ServiceState {
    /// Shared cache.
    pub cache: Arc<ShardedCache>,
    /// Resource registry.
    pub registry: Arc<ResourceRegistry>,
    /// Server configuration.
    pub config: Arc<ServerConfig>,
    /// SotW handler.
    pub sotw: Arc<SotwHandler>,
    /// Delta handler.
    pub delta: Arc<DeltaHandler>,
}

impl ServiceState {
    /// Create new service state.
    pub fn new(
        cache: Arc<ShardedCache>,
        registry: Arc<ResourceRegistry>,
        config: ServerConfig,
    ) -> Self {
        let sotw = Arc::new(SotwHandler::new(Arc::clone(&cache), Arc::clone(&registry)));
        let delta = Arc::new(DeltaHandler::new(Arc::clone(&cache), Arc::clone(&registry)));

        Self {
            cache,
            registry,
            config: Arc::new(config),
            sotw,
            delta,
        }
    }
}

// TODO: Implement tonic services when xds-types is available
// The services will follow this pattern:
//
// pub struct AdsService {
//     state: ServiceState,
// }
//
// #[tonic::async_trait]
// impl AggregatedDiscoveryService for AdsService {
//     async fn stream_aggregated_resources(
//         &self,
//         request: tonic::Request<tonic::Streaming<DiscoveryRequest>>,
//     ) -> Result<tonic::Response<Self::StreamAggregatedResourcesStream>, tonic::Status> {
//         // Implementation
//     }
// }
