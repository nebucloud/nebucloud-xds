//! Listener Discovery Service (LDS) implementation.
//!
//! LDS provides listener configuration to Envoy proxies.

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

/// Listener Discovery Service.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LdsService {
    /// Shared cache.
    cache: Arc<ShardedCache>,
    /// Resource registry.
    registry: Arc<ResourceRegistry>,
    /// SotW handler.
    sotw_handler: Arc<SotwHandler>,
}

impl LdsService {
    /// Create a new LDS service.
    pub fn new(cache: Arc<ShardedCache>, registry: Arc<ResourceRegistry>) -> Self {
        let sotw_handler = Arc::new(SotwHandler::new(Arc::clone(&cache), Arc::clone(&registry)));
        Self {
            cache,
            registry,
            sotw_handler,
        }
    }

    /// Get the type URL for listeners.
    #[inline]
    pub fn type_url() -> &'static str {
        TypeUrl::LISTENER
    }
}

/// Trait for LDS service implementation.
#[async_trait]
pub trait ListenerDiscoveryService: Send + Sync + 'static {
    /// Server streaming response type.
    type StreamListenersStream: Stream<Item = Result<DiscoveryResponse, Status>> + Send + 'static;

    /// Stream listeners to the client.
    async fn stream_listeners(
        &self,
        request: Request<Streaming<DiscoveryRequest>>,
    ) -> Result<Response<Self::StreamListenersStream>, Status>;

    /// Fetch listeners (unary RPC).
    async fn fetch_listeners(
        &self,
        request: Request<DiscoveryRequest>,
    ) -> Result<Response<DiscoveryResponse>, Status>;
}

#[async_trait]
impl ListenerDiscoveryService for LdsService {
    type StreamListenersStream = ReceiverStream<Result<DiscoveryResponse, Status>>;

    #[instrument(skip(self, request), name = "lds_stream")]
    async fn stream_listeners(
        &self,
        request: Request<Streaming<DiscoveryRequest>>,
    ) -> Result<Response<Self::StreamListenersStream>, Status> {
        let mut stream = request.into_inner();
        let (tx, rx) = mpsc::channel(16);

        let handler = Arc::clone(&self.sotw_handler);
        let type_url = TypeUrl::LISTENER;
        let mut ctx = StreamContext::new();

        info!(stream = %ctx.id(), "LDS stream started");

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
                                error!(stream = %ctx.id(), error = %e, "LDS request failed");
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

            info!(stream = %ctx.id(), "LDS stream ended");
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    #[instrument(skip(self, request), name = "lds_fetch")]
    async fn fetch_listeners(
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

        debug!(node = ?node_hash, "LDS fetch request");

        let response = self
            .sotw_handler
            .process_request(
                &ctx,
                TypeUrl::LISTENER.into(),
                &request.version_info,
                &request.resource_names,
                node_hash,
            )
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("no listeners available"))?;

        Ok(Response::new(DiscoveryResponse {
            version_info: response.version_info,
            resources: response
                .resources
                .iter()
                .filter_map(|r| r.encode().ok())
                .collect(),
            type_url: TypeUrl::LISTENER.to_string(),
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
    fn lds_service_creation() {
        let cache = Arc::new(ShardedCache::new());
        let registry = Arc::new(ResourceRegistry::new());
        let _service = LdsService::new(cache, registry);
    }

    #[test]
    fn lds_type_url() {
        assert_eq!(LdsService::type_url(), TypeUrl::LISTENER);
    }
}
