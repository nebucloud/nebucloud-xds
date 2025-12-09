//! Route Discovery Service (RDS) implementation.
//!
//! RDS provides route configuration to Envoy proxies.

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

/// Route Discovery Service.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RdsService {
    /// Shared cache.
    cache: Arc<ShardedCache>,
    /// Resource registry.
    registry: Arc<ResourceRegistry>,
    /// SotW handler.
    sotw_handler: Arc<SotwHandler>,
}

impl RdsService {
    /// Create a new RDS service.
    pub fn new(cache: Arc<ShardedCache>, registry: Arc<ResourceRegistry>) -> Self {
        let sotw_handler = Arc::new(SotwHandler::new(Arc::clone(&cache), Arc::clone(&registry)));
        Self {
            cache,
            registry,
            sotw_handler,
        }
    }

    /// Get the type URL for routes.
    #[inline]
    pub fn type_url() -> &'static str {
        TypeUrl::ROUTE
    }

    /// Convert this service into a tonic service for use with Server::add_service.
    pub fn into_service(self) -> RdsServiceServer {
        RdsServiceServer { inner: self }
    }
}

/// Trait for RDS service implementation.
#[async_trait]
pub trait RouteDiscoveryService: Send + Sync + 'static {
    /// Server streaming response type.
    type StreamRoutesStream: Stream<Item = Result<DiscoveryResponse, Status>> + Send + 'static;

    /// Stream routes to the client.
    async fn stream_routes(
        &self,
        request: Request<Streaming<DiscoveryRequest>>,
    ) -> Result<Response<Self::StreamRoutesStream>, Status>;

    /// Fetch routes (unary RPC).
    async fn fetch_routes(
        &self,
        request: Request<DiscoveryRequest>,
    ) -> Result<Response<DiscoveryResponse>, Status>;
}

#[async_trait]
impl RouteDiscoveryService for RdsService {
    type StreamRoutesStream = ReceiverStream<Result<DiscoveryResponse, Status>>;

    #[instrument(skip(self, request), name = "rds_stream")]
    async fn stream_routes(
        &self,
        request: Request<Streaming<DiscoveryRequest>>,
    ) -> Result<Response<Self::StreamRoutesStream>, Status> {
        let mut stream = request.into_inner();
        let (tx, rx) = mpsc::channel(16);

        let handler = Arc::clone(&self.sotw_handler);
        let type_url = TypeUrl::ROUTE;
        let mut ctx = StreamContext::new();

        info!(stream = %ctx.id(), "RDS stream started");

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
                                error!(stream = %ctx.id(), error = %e, "RDS request failed");
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

            info!(stream = %ctx.id(), "RDS stream ended");
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    #[instrument(skip(self, request), name = "rds_fetch")]
    async fn fetch_routes(
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

        debug!(node = ?node_hash, "RDS fetch request");

        let response = self
            .sotw_handler
            .process_request(
                &ctx,
                TypeUrl::ROUTE.into(),
                &request.version_info,
                &request.resource_names,
                node_hash,
            )
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("no routes available"))?;

        Ok(Response::new(DiscoveryResponse {
            version_info: response.version_info,
            resources: response
                .resources
                .iter()
                .filter_map(|r| r.encode().ok())
                .collect(),
            type_url: TypeUrl::ROUTE.to_string(),
            nonce: response.nonce,
            canary: false,
            control_plane: None,
        }))
    }
}

/// Server wrapper for RdsService.
#[derive(Debug, Clone)]
pub struct RdsServiceServer {
    #[allow(dead_code)]
    inner: RdsService,
}

impl RdsServiceServer {
    /// Create a new server wrapper.
    pub fn new(service: RdsService) -> Self {
        Self { inner: service }
    }
}

impl tonic::codegen::Service<http::Request<tonic::body::BoxBody>> for RdsServiceServer {
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
        let _ = req;
        Box::pin(async move {
            Ok(http::Response::builder()
                .status(http::StatusCode::NOT_IMPLEMENTED)
                .body(tonic::body::empty_body())
                .unwrap())
        })
    }
}

impl tonic::server::NamedService for RdsServiceServer {
    const NAME: &'static str = "envoy.service.route.v3.RouteDiscoveryService";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rds_service_creation() {
        let cache = Arc::new(ShardedCache::new());
        let registry = Arc::new(ResourceRegistry::new());
        let _service = RdsService::new(cache, registry);
    }

    #[test]
    fn rds_type_url() {
        assert_eq!(RdsService::type_url(), TypeUrl::ROUTE);
    }
}
