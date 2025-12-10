//! Shared streaming helpers for xDS discovery services.
//!
//! This module provides common streaming logic that is shared across
//! all discovery services (CDS, EDS, LDS, RDS, SDS) to reduce code
//! duplication and ensure consistent behavior.

use std::sync::Arc;

use tokio::sync::mpsc;
use tonic::{Status, Streaming};
use tracing::{error, info};

use xds_core::{NodeHash, TypeUrl};

use crate::sotw::{SotwHandler, SotwResponse};
use crate::stream::StreamContext;

pub use data_plane_api::envoy::service::discovery::v3::{DiscoveryRequest, DiscoveryResponse};

/// Configuration for a discovery stream.
#[derive(Debug, Clone, Copy)]
pub struct StreamConfig {
    /// The type URL this stream handles (as a static string).
    pub type_url: &'static str,
    /// Service name for logging.
    pub service_name: &'static str,
}

impl StreamConfig {
    /// Create a new stream configuration.
    pub const fn new(type_url: &'static str, service_name: &'static str) -> Self {
        Self {
            type_url,
            service_name,
        }
    }
}

/// Common stream configurations for each discovery service.
pub mod configs {
    use super::*;

    /// Configuration for Cluster Discovery Service.
    pub const CDS: StreamConfig = StreamConfig::new(TypeUrl::CLUSTER, "CDS");
    /// Configuration for Endpoint Discovery Service.
    pub const EDS: StreamConfig = StreamConfig::new(TypeUrl::ENDPOINT, "EDS");
    /// Configuration for Listener Discovery Service.
    pub const LDS: StreamConfig = StreamConfig::new(TypeUrl::LISTENER, "LDS");
    /// Configuration for Route Discovery Service.
    pub const RDS: StreamConfig = StreamConfig::new(TypeUrl::ROUTE, "RDS");
    /// Configuration for Secret Discovery Service.
    pub const SDS: StreamConfig = StreamConfig::new(TypeUrl::SECRET, "SDS");
}

/// Result of processing a stream request.
pub enum StreamAction {
    /// Send a response to the client.
    SendResponse(DiscoveryResponse),
    /// No response needed (client is up to date).
    NoResponse,
    /// An error occurred, send error and break.
    Error(Status),
    /// Stream should break (channel closed, etc).
    Break,
}

/// Handle the common discovery stream logic.
///
/// This function processes incoming discovery requests and sends responses
/// through the provided channel. It handles:
/// - Type URL validation
/// - Node extraction from first request
/// - Request processing via SotwHandler
/// - Response conversion and sending
///
/// # Arguments
///
/// * `stream` - The incoming request stream
/// * `tx` - Channel sender for responses
/// * `handler` - The SotW handler for processing requests
/// * `config` - Stream configuration (type URL, service name)
/// * `convert_response` - Function to convert SotwResponse to DiscoveryResponse
pub async fn handle_discovery_stream<F>(
    mut stream: Streaming<DiscoveryRequest>,
    tx: mpsc::Sender<Result<DiscoveryResponse, Status>>,
    handler: Arc<SotwHandler>,
    config: StreamConfig,
    convert_response: F,
) where
    F: Fn(SotwResponse) -> Result<DiscoveryResponse, Status> + Send + 'static,
{
    let mut ctx = StreamContext::new();
    let mut node_hash: Option<NodeHash> = None;

    info!(
        stream = %ctx.id(),
        service = config.service_name,
        "{} stream started",
        config.service_name
    );

    while let Some(result) = tokio_stream::StreamExt::next(&mut stream).await {
        match result {
            Ok(request) => {
                // Validate type URL
                if !request.type_url.is_empty() && request.type_url != config.type_url {
                    error!(
                        stream = %ctx.id(),
                        expected = config.type_url,
                        got = %request.type_url,
                        "invalid type URL for {}",
                        config.service_name
                    );
                    continue;
                }

                // Extract node info from first request
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
                            service = config.service_name,
                            "first request missing required node information"
                        );
                        let _ = tx
                            .send(Err(Status::invalid_argument(
                                "first request must include node information",
                            )))
                            .await;
                        break;
                    }
                };

                // Process request
                match handler.process_request(
                    &ctx,
                    config.type_url.into(),
                    &request.version_info,
                    &request.resource_names,
                    hash,
                ) {
                    Ok(Some(response)) => match convert_response(response) {
                        Ok(discovery_response) => {
                            if tx.send(Ok(discovery_response)).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            error!(
                                stream = %ctx.id(),
                                error = %e,
                                "failed to convert response"
                            );
                            let _ = tx.send(Err(e)).await;
                            break;
                        }
                    },
                    Ok(None) => {
                        // Client is up to date, no response needed
                    }
                    Err(e) => {
                        error!(
                            stream = %ctx.id(),
                            error = %e,
                            "{} request failed",
                            config.service_name
                        );
                        break;
                    }
                }
            }
            Err(e) => {
                error!(
                    stream = %ctx.id(),
                    error = %e,
                    "stream error"
                );
                break;
            }
        }
    }

    info!(
        stream = %ctx.id(),
        service = config.service_name,
        "{} stream ended",
        config.service_name
    );
}

/// Convert a SotW response to a DiscoveryResponse.
///
/// This is a helper function that handles the common conversion logic
/// for all discovery services.
pub fn convert_sotw_response(
    response: SotwResponse,
    type_url: &str,
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
        type_url: type_url.to_string(),
        nonce: response.nonce,
        canary: false,
        control_plane: None,
        resource_errors: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_config_creation() {
        let config = StreamConfig::new(TypeUrl::CLUSTER, "CDS");
        assert_eq!(config.type_url, TypeUrl::CLUSTER);
        assert_eq!(config.service_name, "CDS");
    }

    #[test]
    fn predefined_configs() {
        assert_eq!(configs::CDS.type_url, TypeUrl::CLUSTER);
        assert_eq!(configs::EDS.type_url, TypeUrl::ENDPOINT);
        assert_eq!(configs::LDS.type_url, TypeUrl::LISTENER);
        assert_eq!(configs::RDS.type_url, TypeUrl::ROUTE);
        assert_eq!(configs::SDS.type_url, TypeUrl::SECRET);
    }
}
