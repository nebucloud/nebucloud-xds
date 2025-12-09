//! Aggregated Discovery Service (ADS) implementation.
//!
//! ADS multiplexes all xDS resource types over a single gRPC stream,
//! ensuring consistent ordering of configuration updates.

use std::sync::Arc;

use async_trait::async_trait;
use futures::Stream;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};
use tracing::{debug, error, info, instrument, warn};

use xds_cache::ShardedCache;
use xds_core::{NodeHash, ResourceRegistry, TypeUrl};

use crate::sotw::{SotwHandler, SotwResponse};
use crate::stream::StreamContext;

/// Configuration for the ADS service.
#[derive(Debug, Clone)]
pub struct AdsConfig {
    /// Maximum concurrent streams per connection.
    pub max_concurrent_streams: usize,
    /// Response buffer size per stream.
    pub response_buffer_size: usize,
    /// Enable delta protocol support.
    pub enable_delta: bool,
}

impl Default for AdsConfig {
    fn default() -> Self {
        Self {
            max_concurrent_streams: 100,
            response_buffer_size: 16,
            enable_delta: true,
        }
    }
}

/// Aggregated Discovery Service.
///
/// Implements the ADS gRPC service, multiplexing CDS, EDS, LDS, RDS, and SDS
/// over a single bidirectional stream.
#[derive(Debug, Clone)]
pub struct AdsService {
    /// Shared cache.
    cache: Arc<ShardedCache>,
    /// Resource registry.
    registry: Arc<ResourceRegistry>,
    /// SotW handler.
    sotw_handler: Arc<SotwHandler>,
    /// Configuration.
    config: AdsConfig,
}

impl AdsService {
    /// Create a new ADS service.
    pub fn new(cache: Arc<ShardedCache>, registry: Arc<ResourceRegistry>) -> Self {
        let sotw_handler = Arc::new(SotwHandler::new(Arc::clone(&cache), Arc::clone(&registry)));
        Self {
            cache,
            registry,
            sotw_handler,
            config: AdsConfig::default(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(
        cache: Arc<ShardedCache>,
        registry: Arc<ResourceRegistry>,
        config: AdsConfig,
    ) -> Self {
        let sotw_handler = Arc::new(SotwHandler::new(Arc::clone(&cache), Arc::clone(&registry)));
        Self {
            cache,
            registry,
            sotw_handler,
            config,
        }
    }

    /// Get a reference to the cache.
    pub fn cache(&self) -> &ShardedCache {
        &self.cache
    }

    /// Get a reference to the registry.
    pub fn registry(&self) -> &ResourceRegistry {
        &self.registry
    }

    /// Get a reference to the configuration.
    pub fn config(&self) -> &AdsConfig {
        &self.config
    }

    /// Convert this service into a tonic service for use with Server::add_service.
    ///
    /// This creates a service that can be added to a tonic router.
    pub fn into_service(self) -> AdsServiceServer {
        AdsServiceServer { inner: self }
    }

    /// Process an incoming SotW discovery request.
    #[allow(clippy::too_many_arguments)]
    #[instrument(skip(self, ctx), fields(stream = %ctx.id()))]
    pub fn process_sotw_request(
        &self,
        ctx: &StreamContext,
        type_url: &str,
        version_info: &str,
        resource_names: &[String],
        node_hash: NodeHash,
        response_nonce: &str,
        error_detail: Option<&str>,
    ) -> Result<Option<DiscoveryResponse>, Status> {
        // Check for NACK
        if let Some(error) = error_detail {
            self.sotw_handler.handle_nack(
                ctx,
                TypeUrl::new(type_url),
                version_info,
                response_nonce,
                error,
            );
            // On NACK, we don't send a new response unless there's new data
        } else if !response_nonce.is_empty() {
            // ACK
            self.sotw_handler
                .handle_ack(ctx, TypeUrl::new(type_url), version_info, response_nonce);
        }

        // Process the request
        let result = self
            .sotw_handler
            .process_request(
                ctx,
                TypeUrl::new(type_url),
                version_info,
                resource_names,
                node_hash,
            )
            .map_err(|e| Status::internal(format!("Failed to process request: {}", e)))?;

        match result {
            Some(response) => Ok(Some(self.convert_sotw_response(response)?)),
            None => Ok(None),
        }
    }

    /// Convert internal SotW response to discovery response.
    #[allow(clippy::result_large_err)]
    fn convert_sotw_response(&self, response: SotwResponse) -> Result<DiscoveryResponse, Status> {
        let resources: Result<Vec<prost_types::Any>, _> = response
            .resources
            .iter()
            .map(|r| {
                r.encode()
                    .map_err(|e| Status::internal(format!("Failed to encode resource: {}", e)))
            })
            .collect();

        Ok(DiscoveryResponse {
            version_info: response.version_info,
            resources: resources?,
            type_url: response.type_url.to_string(),
            nonce: response.nonce,
            canary: false,
            control_plane: None,
        })
    }
}

/// Discovery request for SotW protocol.
#[derive(Debug, Clone, Default)]
pub struct DiscoveryRequest {
    /// Version info from the last response.
    pub version_info: String,
    /// Node information.
    pub node: Option<Node>,
    /// Requested resource names.
    pub resource_names: Vec<String>,
    /// Type URL of requested resources.
    pub type_url: String,
    /// Nonce from the last response.
    pub response_nonce: String,
    /// Error details if this is a NACK.
    pub error_detail: Option<String>,
}

/// Discovery response for SotW protocol.
#[derive(Debug, Clone, Default)]
pub struct DiscoveryResponse {
    /// Version of this response.
    pub version_info: String,
    /// Resources.
    pub resources: Vec<prost_types::Any>,
    /// Whether this is a canary response.
    pub canary: bool,
    /// Type URL of the resources.
    pub type_url: String,
    /// Unique nonce for this response.
    pub nonce: String,
    /// Control plane identifier.
    pub control_plane: Option<ControlPlane>,
}

/// Node information.
#[derive(Debug, Clone, Default)]
pub struct Node {
    /// Node identifier.
    pub id: String,
    /// Cluster the node belongs to.
    pub cluster: String,
}

/// Control plane identifier.
#[derive(Debug, Clone, Default)]
pub struct ControlPlane {
    /// Identifier.
    pub identifier: String,
}

/// Response stream type for ADS.
pub type AdsResponseStream = ReceiverStream<Result<DiscoveryResponse, Status>>;

/// Trait for ADS service implementation.
///
/// This trait defines the interface that the generated tonic code expects.
/// Once we have proper proto generation, this will be replaced by the
/// generated trait.
#[async_trait]
pub trait AggregatedDiscoveryService: Send + Sync + 'static {
    /// Server streaming response type for StreamAggregatedResources.
    type StreamAggregatedResourcesStream: Stream<Item = Result<DiscoveryResponse, Status>>
        + Send
        + 'static;

    /// Bidirectional streaming RPC for xDS.
    async fn stream_aggregated_resources(
        &self,
        request: Request<Streaming<DiscoveryRequest>>,
    ) -> Result<Response<Self::StreamAggregatedResourcesStream>, Status>;

    /// Server streaming response type for DeltaAggregatedResources.
    type DeltaAggregatedResourcesStream: Stream<Item = Result<DeltaDiscoveryResponse, Status>>
        + Send
        + 'static;

    /// Bidirectional streaming RPC for delta xDS.
    async fn delta_aggregated_resources(
        &self,
        request: Request<Streaming<DeltaDiscoveryRequest>>,
    ) -> Result<Response<Self::DeltaAggregatedResourcesStream>, Status>;
}

/// Delta discovery request.
#[derive(Debug, Clone, Default)]
pub struct DeltaDiscoveryRequest {
    /// Node information.
    pub node: Option<Node>,
    /// Type URL of requested resources.
    pub type_url: String,
    /// Resources to subscribe to.
    pub resource_names_subscribe: Vec<String>,
    /// Resources to unsubscribe from.
    pub resource_names_unsubscribe: Vec<String>,
    /// Initial resource versions.
    pub initial_resource_versions: std::collections::HashMap<String, String>,
    /// Nonce from the last response.
    pub response_nonce: String,
    /// Error details if this is a NACK.
    pub error_detail: Option<String>,
}

/// Delta discovery response.
#[derive(Debug, Clone, Default)]
pub struct DeltaDiscoveryResponse {
    /// System version info.
    pub system_version_info: String,
    /// Updated resources.
    pub resources: Vec<Resource>,
    /// Type URL of the resources.
    pub type_url: String,
    /// Removed resource names.
    pub removed_resources: Vec<String>,
    /// Unique nonce for this response.
    pub nonce: String,
    /// Control plane identifier.
    pub control_plane: Option<ControlPlane>,
}

/// A resource in a delta response.
#[derive(Debug, Clone, Default)]
pub struct Resource {
    /// Resource name.
    pub name: String,
    /// Resource version.
    pub version: String,
    /// The resource.
    pub resource: Option<prost_types::Any>,
}

#[async_trait]
impl AggregatedDiscoveryService for AdsService {
    type StreamAggregatedResourcesStream = AdsResponseStream;

    #[instrument(skip(self, request), name = "ads_stream")]
    async fn stream_aggregated_resources(
        &self,
        request: Request<Streaming<DiscoveryRequest>>,
    ) -> Result<Response<Self::StreamAggregatedResourcesStream>, Status> {
        let mut stream = request.into_inner();
        let (tx, rx) = mpsc::channel(self.config.response_buffer_size);

        let service = self.clone();
        let mut ctx = StreamContext::new();

        info!(stream = %ctx.id(), "ADS stream started");

        tokio::spawn(async move {
            let mut node_hash: Option<NodeHash> = None;

            while let Some(result) = tokio_stream::StreamExt::next(&mut stream).await {
                match result {
                    Ok(request) => {
                        // Extract node info on first request
                        if node_hash.is_none() {
                            if let Some(ref node) = request.node {
                                let hash = NodeHash::from_id(&node.id);
                                ctx.set_node(node.id.clone(), hash);
                                node_hash = Some(hash);
                                debug!(
                                    stream = %ctx.id(),
                                    node_id = %node.id,
                                    "node identified"
                                );
                            }
                        }

                        let hash = match node_hash {
                            Some(h) => h,
                            None => {
                                warn!(stream = %ctx.id(), "request without node info");
                                continue;
                            }
                        };

                        // Process the request
                        match service.process_sotw_request(
                            &ctx,
                            &request.type_url,
                            &request.version_info,
                            &request.resource_names,
                            hash,
                            &request.response_nonce,
                            request.error_detail.as_deref(),
                        ) {
                            Ok(Some(response)) => {
                                if tx.send(Ok(response)).await.is_err() {
                                    debug!(stream = %ctx.id(), "client disconnected");
                                    break;
                                }
                            }
                            Ok(None) => {
                                // No update needed
                            }
                            Err(e) => {
                                error!(stream = %ctx.id(), error = %e, "request processing failed");
                                let _ = tx.send(Err(e)).await;
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

            info!(
                stream = %ctx.id(),
                duration = ?ctx.duration(),
                requests = ctx.request_count(),
                responses = ctx.response_count(),
                "ADS stream ended"
            );
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    type DeltaAggregatedResourcesStream = ReceiverStream<Result<DeltaDiscoveryResponse, Status>>;

    #[instrument(skip(self, request), name = "ads_delta_stream")]
    async fn delta_aggregated_resources(
        &self,
        request: Request<Streaming<DeltaDiscoveryRequest>>,
    ) -> Result<Response<Self::DeltaAggregatedResourcesStream>, Status> {
        let mut stream = request.into_inner();
        let (tx, rx) = mpsc::channel(self.config.response_buffer_size);

        let ctx = StreamContext::new();
        info!(stream = %ctx.id(), "Delta ADS stream started");

        // TODO: Implement delta protocol handling
        // For now, just log and close
        tokio::spawn(async move {
            while let Some(result) = tokio_stream::StreamExt::next(&mut stream).await {
                match result {
                    Ok(request) => {
                        debug!(
                            stream = %ctx.id(),
                            type_url = %request.type_url,
                            "delta request received (not yet implemented)"
                        );
                    }
                    Err(e) => {
                        error!(stream = %ctx.id(), error = %e, "delta stream error");
                        break;
                    }
                }
            }

            info!(stream = %ctx.id(), "Delta ADS stream ended");
            drop(tx);
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

/// Server wrapper for AdsService that implements tonic service traits.
///
/// This provides the gRPC service implementation that can be added to a tonic router.
#[derive(Debug, Clone)]
pub struct AdsServiceServer {
    inner: AdsService,
}

impl AdsServiceServer {
    /// Create a new server wrapper.
    pub fn new(service: AdsService) -> Self {
        Self { inner: service }
    }

    /// Get a reference to the inner service.
    pub fn inner(&self) -> &AdsService {
        &self.inner
    }
}

// Note: In a full implementation, this would implement the tonic-generated
// service trait. For now, we provide a placeholder that allows the server
// to be composed with other services.

impl tonic::codegen::Service<http::Request<tonic::body::BoxBody>> for AdsServiceServer {
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
        // This is a placeholder - in production, this would route to the
        // appropriate gRPC method based on the request path
        let _ = req;
        Box::pin(async move {
            Ok(http::Response::builder()
                .status(http::StatusCode::NOT_IMPLEMENTED)
                .body(tonic::body::empty_body())
                .unwrap())
        })
    }
}

impl tonic::server::NamedService for AdsServiceServer {
    const NAME: &'static str = "envoy.service.discovery.v3.AggregatedDiscoveryService";
}

#[cfg(test)]
mod tests {
    use super::*;
    use xds_cache::{Cache, Snapshot};

    fn setup() -> AdsService {
        let cache = Arc::new(ShardedCache::new());
        let registry = Arc::new(ResourceRegistry::new());
        AdsService::new(cache, registry)
    }

    #[test]
    fn ads_service_creation() {
        let service = setup();
        assert!(service.cache().snapshot_count() == 0);
    }

    #[test]
    fn ads_service_with_config() {
        let cache = Arc::new(ShardedCache::new());
        let registry = Arc::new(ResourceRegistry::new());
        let config = AdsConfig {
            max_concurrent_streams: 50,
            response_buffer_size: 8,
            enable_delta: false,
        };

        let service = AdsService::with_config(cache, registry, config);
        assert!(!service.config.enable_delta);
    }

    #[test]
    fn process_request_no_snapshot() {
        let service = setup();
        let ctx = StreamContext::new();
        let node_hash = NodeHash::from_id("unknown-node");

        let result = service
            .process_sotw_request(
                &ctx,
                "type.googleapis.com/test",
                "",
                &[],
                node_hash,
                "",
                None,
            )
            .unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn process_request_with_snapshot() {
        let service = setup();
        let ctx = StreamContext::new();
        let node_hash = NodeHash::from_id("test-node");

        // Add a snapshot
        let snapshot = Snapshot::builder()
            .version("v1")
            .resources(TypeUrl::CLUSTER.into(), vec![])
            .build();
        service.cache().set_snapshot(node_hash, snapshot);

        // Request should return a response (empty resources but valid)
        let result = service
            .process_sotw_request(&ctx, TypeUrl::CLUSTER, "", &[], node_hash, "", None)
            .unwrap();

        assert!(result.is_some());
        let response = result.unwrap();
        assert_eq!(response.version_info, "v1");
    }
}
