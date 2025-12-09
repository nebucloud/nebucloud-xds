//! Secret Discovery Service (SDS) implementation.
//!
//! SDS provides TLS certificate and key configuration to Envoy proxies.

use std::sync::Arc;

use async_trait::async_trait;
use futures::Stream;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};
use tracing::{debug, error, info, instrument};

use xds_cache::ShardedCache;
use xds_core::{NodeHash, ResourceRegistry, TypeUrl};

use crate::sotw::SotwHandler;
use crate::stream::StreamContext;

use super::ads::{DiscoveryRequest, DiscoveryResponse};

/// Secret Discovery Service.
#[derive(Debug, Clone)]
#[allow(dead_code)]
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
}

/// Trait for SDS service implementation.
#[async_trait]
pub trait SecretDiscoveryService: Send + Sync + 'static {
    /// Server streaming response type.
    type StreamSecretsStream: Stream<Item = Result<DiscoveryResponse, Status>> + Send + 'static;

    /// Stream secrets to the client.
    async fn stream_secrets(
        &self,
        request: Request<Streaming<DiscoveryRequest>>,
    ) -> Result<Response<Self::StreamSecretsStream>, Status>;

    /// Fetch secrets (unary RPC).
    async fn fetch_secrets(
        &self,
        request: Request<DiscoveryRequest>,
    ) -> Result<Response<DiscoveryResponse>, Status>;
}

#[async_trait]
impl SecretDiscoveryService for SdsService {
    type StreamSecretsStream = ReceiverStream<Result<DiscoveryResponse, Status>>;

    #[instrument(skip(self, request), name = "sds_stream")]
    async fn stream_secrets(
        &self,
        request: Request<Streaming<DiscoveryRequest>>,
    ) -> Result<Response<Self::StreamSecretsStream>, Status> {
        let mut stream = request.into_inner();
        let (tx, rx) = mpsc::channel(16);

        let handler = Arc::clone(&self.sotw_handler);
        let type_url = TypeUrl::SECRET;
        let mut ctx = StreamContext::new();

        info!(stream = %ctx.id(), "SDS stream started");

        tokio::spawn(async move {
            let mut node_hash: Option<NodeHash> = None;

            while let Some(result) = tokio_stream::StreamExt::next(&mut stream).await {
                match result {
                    Ok(request) => {
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

                        match handler.process_request(
                            &ctx,
                            type_url.into(),
                            &request.version_info,
                            &request.resource_names,
                            hash,
                        ) {
                            Ok(Some(response)) => {
                                let discovery_response = DiscoveryResponse {
                                    version_info: response.version_info,
                                    resources: response
                                        .resources
                                        .iter()
                                        .filter_map(|r| r.encode().ok())
                                        .collect(),
                                    type_url: type_url.to_string(),
                                    nonce: response.nonce,
                                    canary: false,
                                    control_plane: None,
                                };
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

        Ok(Response::new(DiscoveryResponse {
            version_info: response.version_info,
            resources: response
                .resources
                .iter()
                .filter_map(|r| r.encode().ok())
                .collect(),
            type_url: TypeUrl::SECRET.to_string(),
            nonce: response.nonce,
            canary: false,
            control_plane: None,
        }))
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
