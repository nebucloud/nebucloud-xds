//! Endpoint Discovery Service (EDS) implementation.
//!
//! EDS provides endpoint configuration to Envoy proxies.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};
use tracing::{debug, error, info, instrument};

use xds_cache::ShardedCache;
use xds_core::{NodeHash, ResourceRegistry, TypeUrl};

use crate::sotw::SotwHandler;
use crate::stream::StreamContext;

// Re-export the data-plane-api types
pub use data_plane_api::envoy::service::discovery::v3::{DiscoveryRequest, DiscoveryResponse};
pub use data_plane_api::envoy::service::endpoint::v3::endpoint_discovery_service_server::{
    EndpointDiscoveryService, EndpointDiscoveryServiceServer,
};
pub use data_plane_api::envoy::service::discovery::v3::{
    DeltaDiscoveryRequest, DeltaDiscoveryResponse,
};

/// Endpoint Discovery Service.
#[derive(Debug, Clone)]
pub struct EdsService {
    /// Shared cache.
    cache: Arc<ShardedCache>,
    /// Resource registry.
    registry: Arc<ResourceRegistry>,
    /// SotW handler.
    sotw_handler: Arc<SotwHandler>,
}

impl EdsService {
    /// Create a new EDS service.
    pub fn new(cache: Arc<ShardedCache>, registry: Arc<ResourceRegistry>) -> Self {
        let sotw_handler = Arc::new(SotwHandler::new(Arc::clone(&cache), Arc::clone(&registry)));
        Self {
            cache,
            registry,
            sotw_handler,
        }
    }

    /// Get the type URL for endpoints.
    #[inline]
    pub fn type_url() -> &'static str {
        TypeUrl::ENDPOINT
    }

    /// Get a reference to the cache.
    #[allow(dead_code)]
    pub fn cache(&self) -> &ShardedCache {
        &self.cache
    }

    /// Get a reference to the registry.
    #[allow(dead_code)]
    pub fn registry(&self) -> &ResourceRegistry {
        &self.registry
    }

    /// Convert this service into a tonic service for use with Server::add_service.
    pub fn into_service(self) -> EndpointDiscoveryServiceServer<Self> {
        EndpointDiscoveryServiceServer::new(self)
    }

    /// Convert a SotW response to a proto DiscoveryResponse.
    ///
    /// Returns an error if any resource fails to encode, rather than
    /// silently dropping it.
    fn convert_response(
        &self,
        response: crate::sotw::SotwResponse,
    ) -> Result<DiscoveryResponse, Status> {
        use data_plane_api::google::protobuf::Any;

        let resources: Vec<Any> = response
            .resources
            .iter()
            .map(|r| {
                r.encode().map(|encoded| Any {
                    type_url: encoded.type_url.clone(),
                    value: encoded.value.clone(),
                })
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| Status::internal(format!("failed to encode resource: {}", e)))?;

        Ok(DiscoveryResponse {
            version_info: response.version_info,
            resources,
            type_url: TypeUrl::ENDPOINT.to_string(),
            nonce: response.nonce,
            canary: false,
            control_plane: None,
            resource_errors: vec![],
        })
    }
}

/// Response stream type for EDS.
pub type EdsResponseStream = ReceiverStream<Result<DiscoveryResponse, Status>>;

/// Delta response stream type for EDS.
pub type EdsDeltaResponseStream = ReceiverStream<Result<DeltaDiscoveryResponse, Status>>;

#[async_trait]
impl EndpointDiscoveryService for EdsService {
    type StreamEndpointsStream = EdsResponseStream;

    #[instrument(skip(self, request), name = "eds_stream")]
    async fn stream_endpoints(
        &self,
        request: Request<Streaming<DiscoveryRequest>>,
    ) -> Result<Response<Self::StreamEndpointsStream>, Status> {
        let mut stream = request.into_inner();
        let (tx, rx) = mpsc::channel(16);

        let service = self.clone();
        let mut ctx = StreamContext::new();

        info!(stream = %ctx.id(), "EDS stream started");

        tokio::spawn(async move {
            let mut node_hash: Option<NodeHash> = None;

            while let Some(result) = tokio_stream::StreamExt::next(&mut stream).await {
                match result {
                    Ok(request) => {
                        // Validate type URL
                        if !request.type_url.is_empty() && request.type_url != TypeUrl::ENDPOINT {
                            error!(
                                stream = %ctx.id(),
                                expected = TypeUrl::ENDPOINT,
                                got = %request.type_url,
                                "invalid type URL for EDS"
                            );
                            continue;
                        }

                        // Extract node info
                        if node_hash.is_none() {
                            if let Some(ref node) = request.node {
                                let hash = NodeHash::from_id(&node.id);
                                ctx.set_node(node.id.clone(), hash);
                                node_hash = Some(hash);
                            }
                        }

                        let hash = match node_hash {
                            Some(h) => h,
                            None => {
                                // First request must include node information
                                error!(
                                    stream = %ctx.id(),
                                    "first request missing required node information"
                                );
                                let _ = tx.send(Err(Status::invalid_argument(
                                    "first request must include node information"
                                ))).await;
                                break;
                            }
                        };

                        // Process request
                        match service.sotw_handler.process_request(
                            &ctx,
                            TypeUrl::ENDPOINT.into(),
                            &request.version_info,
                            &request.resource_names,
                            hash,
                        ) {
                            Ok(Some(response)) => {
                                match service.convert_response(response) {
                                    Ok(discovery_response) => {
                                        if tx.send(Ok(discovery_response)).await.is_err() {
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        error!(stream = %ctx.id(), error = %e, "failed to convert response");
                                        let _ = tx.send(Err(e)).await;
                                        break;
                                    }
                                }
                            }
                            Ok(None) => {}
                            Err(e) => {
                                error!(stream = %ctx.id(), error = %e, "EDS request failed");
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error!(stream = %ctx.id(), error = %e, "stream error");
                        break;
                    }
                }
            }

            info!(stream = %ctx.id(), "EDS stream ended");
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    type DeltaEndpointsStream = EdsDeltaResponseStream;

    #[instrument(skip(self, request), name = "eds_delta_stream")]
    async fn delta_endpoints(
        &self,
        request: Request<Streaming<DeltaDiscoveryRequest>>,
    ) -> Result<Response<Self::DeltaEndpointsStream>, Status> {
        let mut stream = request.into_inner();
        let (tx, rx) = mpsc::channel(16);

        let ctx = StreamContext::new();
        info!(stream = %ctx.id(), "Delta EDS stream started");

        tokio::spawn(async move {
            while let Some(result) = tokio_stream::StreamExt::next(&mut stream).await {
                match result {
                    Ok(request) => {
                        debug!(
                            stream = %ctx.id(),
                            type_url = %request.type_url,
                            "delta EDS request received (not yet implemented)"
                        );
                    }
                    Err(e) => {
                        error!(stream = %ctx.id(), error = %e, "delta stream error");
                        break;
                    }
                }
            }

            info!(stream = %ctx.id(), "Delta EDS stream ended");
            drop(tx);
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    #[instrument(skip(self, request), name = "eds_fetch")]
    async fn fetch_endpoints(
        &self,
        request: Request<DiscoveryRequest>,
    ) -> Result<Response<DiscoveryResponse>, Status> {
        let request = request.into_inner();
        let ctx = StreamContext::new();

        let node_hash = request
            .node
            .as_ref()
            .map(|n| NodeHash::from_id(&n.id))
            .ok_or_else(|| Status::invalid_argument("node is required"))?;

        debug!(node = ?node_hash, "EDS fetch request");

        let response = self
            .sotw_handler
            .process_request(
                &ctx,
                TypeUrl::ENDPOINT.into(),
                &request.version_info,
                &request.resource_names,
                node_hash,
            )
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("no endpoints available"))?;

        Ok(Response::new(self.convert_response(response)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eds_service_creation() {
        let cache = Arc::new(ShardedCache::new());
        let registry = Arc::new(ResourceRegistry::new());
        let _service = EdsService::new(cache, registry);
    }

    #[test]
    fn eds_type_url() {
        assert_eq!(EdsService::type_url(), TypeUrl::ENDPOINT);
    }
}
