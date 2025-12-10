//! gRPC service implementations for xDS.
//!
//! This module contains the tonic service implementations for:
//! - Aggregated Discovery Service (ADS)
//! - Cluster Discovery Service (CDS)
//! - Listener Discovery Service (LDS)
//! - Route Discovery Service (RDS)
//! - Endpoint Discovery Service (EDS)
//! - Secret Discovery Service (SDS)

pub mod ads;
pub mod cds;
pub mod eds;
pub mod lds;
pub mod rds;
pub mod sds;

use std::sync::Arc;

use xds_cache::ShardedCache;
use xds_core::ResourceRegistry;

use crate::config::ServerConfig;
use crate::delta::DeltaHandler;
use crate::sotw::SotwHandler;

// Re-export all services and traits
pub use ads::{AdsConfig, AdsService, AggregatedDiscoveryService, AggregatedDiscoveryServiceServer};
pub use cds::{CdsService, ClusterDiscoveryService, ClusterDiscoveryServiceServer};
pub use eds::{EdsService, EndpointDiscoveryService, EndpointDiscoveryServiceServer};
pub use lds::{LdsService, ListenerDiscoveryService, ListenerDiscoveryServiceServer};
pub use rds::{RdsService, RouteDiscoveryService, RouteDiscoveryServiceServer};
pub use sds::{SdsService, SecretDiscoveryService, SecretDiscoveryServiceServer};

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

    /// Create all discovery services from this state.
    ///
    /// Services share the SotW handler from this state for better resource
    /// efficiency instead of creating individual handlers.
    pub fn create_services(
        &self,
    ) -> (
        AdsService,
        CdsService,
        LdsService,
        RdsService,
        EdsService,
        SdsService,
    ) {
        (
            AdsService::new(Arc::clone(&self.cache), Arc::clone(&self.registry)),
            CdsService::new_with_handler(
                Arc::clone(&self.cache),
                Arc::clone(&self.registry),
                Arc::clone(&self.sotw),
            ),
            LdsService::new_with_handler(
                Arc::clone(&self.cache),
                Arc::clone(&self.registry),
                Arc::clone(&self.sotw),
            ),
            RdsService::new_with_handler(
                Arc::clone(&self.cache),
                Arc::clone(&self.registry),
                Arc::clone(&self.sotw),
            ),
            EdsService::new_with_handler(
                Arc::clone(&self.cache),
                Arc::clone(&self.registry),
                Arc::clone(&self.sotw),
            ),
            SdsService::new_with_handler(
                Arc::clone(&self.cache),
                Arc::clone(&self.registry),
                Arc::clone(&self.sotw),
            ),
        )
    }
}
