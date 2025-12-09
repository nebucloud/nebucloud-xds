//! Secret Discovery Service (SDS) implementation.
//!
//! SDS provides secret (TLS certificate) configuration to Envoy proxies.

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
pub use data_plane_api::envoy::service::secret::v3::secret_discovery_service_server::{
    SecretDiscoveryService, SecretDiscoveryServiceServer,
};
pub use data_plane_api::envoy::service::discovery::v3::{
    DeltaDiscoveryRequest, DeltaDiscoveryResponse,
};

/// Secret Discovery Service.
#[derive(Debug, Clone)]
pub struct SdsService {
    /// Shared cache.
    cache: Arc<ShardedCache>,
    /// Resource registry.
    registry: Arc<ResourceRegistry>,
    /// SotW handler.
    sotw_handler: Arc<SotwHandler>,
}

impl SdsService {
    /// Create a new SDS service.
    pub fn new(cache: Arc<ShardedCache>, registry: Arc<ResourceRegistry>) -> Self {
        let sotw_handler = Arc::new(SotwHandler::new(Arc::clone(&cache), Arc::clone(&registry)));
        Self {
            cache,
            registry,
            sotw_handler,
        }
    }

    /// Get the type URL for secrets.
    #[inline]
    pub fn type_url() -> &'static str {
        TypeUrl::SECRET
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
    pub fn into_service(self) -> SecretDiscoveryServiceServer<Self> {
        SecretDiscoveryServiceServer::new(self)
    }

    /// Convert a SotW response to a proto DiscoveryResponse.
    fn convert_response(
        &self,
        response: crate::sotw::SotwResponse,
    ) -> DiscoveryResponse {
        use data_plane_api::google::protobuf::Any;

        let resources: Vec<Any> = response
            .resources
            .iter()
            .filter_map(|r| {
                r.encode().ok().map(|encoded| Any {
                    type_url: encoded.type_url.clone(),
                    value: encoded.value.clone(),
                })
            })
            .collect();

        DiscoveryResponse {
            version_info: response.version_info,
            resources,
            type_url: TypeUrl::SECRET.to_string(),
            nonce: response.nonce,
            canary: false,
            control_plane: None,
            resource_errors: vec![],
        }
    }
}

/// Response stream type for SDS.
pub type SdsResponseStream = ReceiverStream<Result<DiscoveryResponse, Status>>;

/// Delta response stream type for SDS.
pub type SdsDeltaResponseStream = ReceiverStream<Result<DeltaDiscoveryResponse, Status>>;

#[async_trait]
impl SecretDiscoveryService for SdsService {
    type StreamSecretsStream = SdsResponseStream;

    #[instrument(skip(self, request), name = "sds_stream")]
    async fn stream_secrets(
        &self,
        request: Request<Streaming<DiscoveryRequest>>,
    ) -> Result<Response<Self::StreamSecretsStream>, Status> {
        let mut stream = request.into_inner();
        let (tx, rx) = mpsc::channel(16);

        let service = self.clone();
        let mut ctx = StreamContext::new();

        info!(stream = %ctx.id(), "SDS stream started");

        tokio::spawn(async move {
            let mut node_hash: Option<NodeHash> = None;

            while let Some(result) = tokio_stream::StreamExt::next(&mut stream).await {
                match result {
                    Ok(request) => {
                        // Validate type URL
                        if !request.type_url.is_empty() && request.type_url != TypeUrl::SECRET {
                            error!(
                                stream = %ctx.id(),
                                expected = TypeUrl::SECRET,
                                got = %request.type_url,
                                "invalid type URL for SDS"
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
                            None => continue,
                        };

                        // Process request
                        match service.sotw_handler.process_request(
                            &ctx,
                            TypeUrl::SECRET.into(),
                            &request.version_info,
                            &request.resource_names,
                            hash,
                        ) {
                            Ok(Some(response)) => {
                                let discovery_response = service.convert_response(response);
                                if tx.send(Ok(discovery_response)).await.is_err() {
                                    break;
                                }
                            }
                            Ok(None) => {}
                            Err(e) => {
                                error!(stream = %ctx.id(), error = %e, "SDS request failed");
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

            info!(stream = %ctx.id(), "SDS stream ended");
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    type DeltaSecretsStream = SdsDeltaResponseStream;

    #[instrument(skip(self, request), name = "sds_delta_stream")]
    async fn delta_secrets(
        &self,
        request: Request<Streaming<DeltaDiscoveryRequest>>,
    ) -> Result<Response<Self::DeltaSecretsStream>, Status> {
        let mut stream = request.into_inner();
        let (tx, rx) = mpsc::channel(16);

        let ctx = StreamContext::new();
        info!(stream = %ctx.id(), "Delta SDS stream started");

        tokio::spawn(async move {
            while let Some(result) = tokio_stream::StreamExt::next(&mut stream).await {
                match result {
                    Ok(request) => {
                        debug!(
                            stream = %ctx.id(),
                            type_url = %request.type_url,
                            "delta SDS request received (not yet implemented)"
                        );
                    }
                    Err(e) => {
                        error!(stream = %ctx.id(), error = %e, "delta stream error");
                        break;
                    }
                }
            }

            info!(stream = %ctx.id(), "Delta SDS stream ended");
            drop(tx);
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    #[instrument(skip(self, request), name = "sds_fetch")]
    async fn fetch_secrets(
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

        debug!(node = ?node_hash, "SDS fetch request");

        let response = self
            .sotw_handler
            .process_request(
                &ctx,
                TypeUrl::SECRET.into(),
                &request.version_info,
                &request.resource_names,
                node_hash,
            )
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("no secrets available"))?;

        Ok(Response::new(self.convert_response(response)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sds_service_creation() {
        let cache = Arc::new(ShardedCache::new());
        let registry = Arc::new(ResourceRegistry::new());
        let _service = SdsService::new(cache, registry);
    }

    #[test]
    fn sds_type_url() {
        assert_eq!(SdsService::type_url(), TypeUrl::SECRET);
    }
}
