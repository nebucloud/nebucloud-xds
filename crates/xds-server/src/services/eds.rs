//! Endpoint Discovery Service (EDS) implementation.
//!
//! EDS provides endpoint (cluster member) configuration to Envoy proxies.

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

/// Endpoint Discovery Service.
#[derive(Debug, Clone)]
#[allow(dead_code)]
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

    /// Convert this service into a tonic service for use with Server::add_service.
    pub fn into_service(self) -> EdsServiceServer {
        EdsServiceServer { inner: self }
    }
}

/// Trait for EDS service implementation.
#[async_trait]
pub trait EndpointDiscoveryService: Send + Sync + 'static {
    /// Server streaming response type.
    type StreamEndpointsStream: Stream<Item = Result<DiscoveryResponse, Status>> + Send + 'static;

    /// Stream endpoints to the client.
    async fn stream_endpoints(
        &self,
        request: Request<Streaming<DiscoveryRequest>>,
    ) -> Result<Response<Self::StreamEndpointsStream>, Status>;

    /// Fetch endpoints (unary RPC).
    async fn fetch_endpoints(
        &self,
        request: Request<DiscoveryRequest>,
    ) -> Result<Response<DiscoveryResponse>, Status>;
}

#[async_trait]
impl EndpointDiscoveryService for EdsService {
    type StreamEndpointsStream = ReceiverStream<Result<DiscoveryResponse, Status>>;

    #[instrument(skip(self, request), name = "eds_stream")]
    async fn stream_endpoints(
        &self,
        request: Request<Streaming<DiscoveryRequest>>,
    ) -> Result<Response<Self::StreamEndpointsStream>, Status> {
        let mut stream = request.into_inner();
        let (tx, rx) = mpsc::channel(16);

        let handler = Arc::clone(&self.sotw_handler);
        let type_url = TypeUrl::ENDPOINT;
        let mut ctx = StreamContext::new();

        info!(stream = %ctx.id(), "EDS stream started");

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

        Ok(Response::new(DiscoveryResponse {
            version_info: response.version_info,
            resources: response
                .resources
                .iter()
                .filter_map(|r| r.encode().ok())
                .collect(),
            type_url: TypeUrl::ENDPOINT.to_string(),
            nonce: response.nonce,
            canary: false,
            control_plane: None,
        }))
    }
}

/// Server wrapper for EdsService.
#[derive(Debug, Clone)]
pub struct EdsServiceServer {
    #[allow(dead_code)]
    inner: EdsService,
}

impl EdsServiceServer {
    /// Create a new server wrapper.
    pub fn new(service: EdsService) -> Self {
        Self { inner: service }
    }
}

impl tonic::codegen::Service<http::Request<tonic::body::BoxBody>> for EdsServiceServer {
    type Response = http::Response<tonic::body::BoxBody>;
    type Error = std::convert::Infallible;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::Request<tonic::body::BoxBody>) -> Self::Future {
        let path = req.uri().path().to_string();
        tracing::warn!(
            path = %path,
            "EDS service called but proto integration not yet complete"
        );
        Box::pin(async move {
            let status = tonic::Status::unimplemented(
                "EDS service requires integration with data-plane-api generated types"
            );
            Ok(status.into_http())
        })
    }
}

impl tonic::server::NamedService for EdsServiceServer {
    const NAME: &'static str = "envoy.service.endpoint.v3.EndpointDiscoveryService";
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
