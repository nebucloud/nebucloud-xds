//! Cluster Discovery Service (CDS) implementation.
//!
//! CDS provides cluster configuration to Envoy proxies.

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

/// Cluster Discovery Service.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CdsService {
    /// Shared cache.
    cache: Arc<ShardedCache>,
    /// Resource registry.
    registry: Arc<ResourceRegistry>,
    /// SotW handler.
    sotw_handler: Arc<SotwHandler>,
}

impl CdsService {
    /// Create a new CDS service.
    pub fn new(cache: Arc<ShardedCache>, registry: Arc<ResourceRegistry>) -> Self {
        let sotw_handler = Arc::new(SotwHandler::new(Arc::clone(&cache), Arc::clone(&registry)));
        Self {
            cache,
            registry,
            sotw_handler,
        }
    }

    /// Get the type URL for clusters.
    #[inline]
    pub fn type_url() -> &'static str {
        TypeUrl::CLUSTER
    }

    /// Convert this service into a tonic service for use with Server::add_service.
    pub fn into_service(self) -> CdsServiceServer {
        CdsServiceServer { inner: self }
    }
}

/// Trait for CDS service implementation.
#[async_trait]
pub trait ClusterDiscoveryService: Send + Sync + 'static {
    /// Server streaming response type.
    type StreamClustersStream: Stream<Item = Result<DiscoveryResponse, Status>> + Send + 'static;

    /// Stream clusters to the client.
    async fn stream_clusters(
        &self,
        request: Request<Streaming<DiscoveryRequest>>,
    ) -> Result<Response<Self::StreamClustersStream>, Status>;

    /// Fetch clusters (unary RPC).
    async fn fetch_clusters(
        &self,
        request: Request<DiscoveryRequest>,
    ) -> Result<Response<DiscoveryResponse>, Status>;
}

#[async_trait]
impl ClusterDiscoveryService for CdsService {
    type StreamClustersStream = ReceiverStream<Result<DiscoveryResponse, Status>>;

    #[instrument(skip(self, request), name = "cds_stream")]
    async fn stream_clusters(
        &self,
        request: Request<Streaming<DiscoveryRequest>>,
    ) -> Result<Response<Self::StreamClustersStream>, Status> {
        let mut stream = request.into_inner();
        let (tx, rx) = mpsc::channel(16);

        let handler = Arc::clone(&self.sotw_handler);
        let mut ctx = StreamContext::new();

        info!(stream = %ctx.id(), "CDS stream started");

        tokio::spawn(async move {
            let mut node_hash: Option<NodeHash> = None;

            while let Some(result) = tokio_stream::StreamExt::next(&mut stream).await {
                match result {
                    Ok(request) => {
                        // Validate type URL
                        if !request.type_url.is_empty() && request.type_url != TypeUrl::CLUSTER {
                            error!(
                                stream = %ctx.id(),
                                expected = TypeUrl::CLUSTER,
                                got = %request.type_url,
                                "invalid type URL for CDS"
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
                        match handler.process_request(
                            &ctx,
                            TypeUrl::CLUSTER.into(),
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
                                    type_url: TypeUrl::CLUSTER.to_string(),
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
                                error!(stream = %ctx.id(), error = %e, "CDS request failed");
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

            info!(stream = %ctx.id(), "CDS stream ended");
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    #[instrument(skip(self, request), name = "cds_fetch")]
    async fn fetch_clusters(
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

        debug!(node = ?node_hash, "CDS fetch request");

        let response = self
            .sotw_handler
            .process_request(
                &ctx,
                TypeUrl::CLUSTER.into(),
                &request.version_info,
                &request.resource_names,
                node_hash,
            )
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("no clusters available"))?;

        Ok(Response::new(DiscoveryResponse {
            version_info: response.version_info,
            resources: response
                .resources
                .iter()
                .filter_map(|r| r.encode().ok())
                .collect(),
            type_url: TypeUrl::CLUSTER.to_string(),
            nonce: response.nonce,
            canary: false,
            control_plane: None,
        }))
    }
}

/// Server wrapper for CdsService.
#[derive(Debug, Clone)]
pub struct CdsServiceServer {
    #[allow(dead_code)]
    inner: CdsService,
}

impl CdsServiceServer {
    /// Create a new server wrapper.
    pub fn new(service: CdsService) -> Self {
        Self { inner: service }
    }
}

impl tonic::codegen::Service<http::Request<tonic::body::BoxBody>> for CdsServiceServer {
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

impl tonic::server::NamedService for CdsServiceServer {
    const NAME: &'static str = "envoy.service.cluster.v3.ClusterDiscoveryService";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cds_service_creation() {
        let cache = Arc::new(ShardedCache::new());
        let registry = Arc::new(ResourceRegistry::new());
        let _service = CdsService::new(cache, registry);
    }

    #[test]
    fn cds_type_url() {
        assert_eq!(CdsService::type_url(), TypeUrl::CLUSTER);
    }
}
