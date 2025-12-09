//! State-of-the-World (SotW) xDS protocol handler.
//!
//! SotW is the original xDS protocol where the entire resource set
//! is sent on each update.

use std::sync::Arc;

use tracing::{debug, info, trace, warn};
use xds_cache::{Cache, ShardedCache};
use xds_core::{NodeHash, ResourceRegistry, TypeUrl, XdsResult};

use crate::stream::StreamContext;
use crate::utils::{generate_nonce, NoncePrefix};

/// Handler for State-of-the-World xDS requests.
///
/// This handler processes DiscoveryRequest messages and produces
/// DiscoveryResponse messages containing the full resource set.
#[derive(Debug)]
pub struct SotwHandler {
    /// Shared cache.
    cache: Arc<ShardedCache>,
    /// Resource registry.
    registry: Arc<ResourceRegistry>,
}

impl SotwHandler {
    /// Create a new SotW handler.
    pub fn new(cache: Arc<ShardedCache>, registry: Arc<ResourceRegistry>) -> Self {
        Self { cache, registry }
    }

    /// Get a reference to the cache.
    #[inline]
    pub fn cache(&self) -> &ShardedCache {
        &self.cache
    }

    /// Get a reference to the registry.
    #[inline]
    pub fn registry(&self) -> &ResourceRegistry {
        &self.registry
    }

    /// Process an incoming discovery request.
    ///
    /// Returns a response with resources if available, or `None` if
    /// the client already has the latest version.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Stream context for this request
    /// * `type_url` - The type of resources being requested
    /// * `version_info` - The version the client currently has (empty on first request)
    /// * `resource_names` - Specific resources requested (empty for wildcard)
    /// * `node_hash` - Hash of the requesting node
    pub fn process_request(
        &self,
        ctx: &StreamContext,
        type_url: TypeUrl,
        version_info: &str,
        resource_names: &[String],
        node_hash: NodeHash,
    ) -> XdsResult<Option<SotwResponse>> {
        ctx.record_request();

        trace!(
            stream = %ctx.id(),
            type_url = %type_url,
            version = %version_info,
            resources = ?resource_names,
            "processing SotW request"
        );

        // Get snapshot for this node
        let snapshot = match self.cache.get_snapshot(node_hash) {
            Some(s) => s,
            None => {
                debug!(
                    stream = %ctx.id(),
                    node = %node_hash,
                    "no snapshot available for node"
                );
                return Ok(None);
            }
        };

        // Get resources of the requested type
        let resources = match snapshot.get_resources(type_url.clone()) {
            Some(r) => r,
            None => {
                debug!(
                    stream = %ctx.id(),
                    type_url = %type_url,
                    "no resources of type in snapshot"
                );
                return Ok(None);
            }
        };

        // Check if client already has this version
        if !version_info.is_empty() && version_info == resources.version() {
            trace!(
                stream = %ctx.id(),
                version = %version_info,
                "client already has latest version"
            );
            return Ok(None);
        }

        // Build response
        let response = if resource_names.is_empty() {
            // Wildcard: send all resources
            SotwResponse {
                version_info: resources.version().to_string(),
                resources: resources.to_vec(),
                type_url,
                nonce: generate_nonce(NoncePrefix::SotW),
            }
        } else {
            // Specific resources requested
            let filtered: Vec<_> = resource_names
                .iter()
                .filter_map(|name| resources.get(name).cloned())
                .collect();

            SotwResponse {
                version_info: resources.version().to_string(),
                resources: filtered,
                type_url,
                nonce: generate_nonce(NoncePrefix::SotW),
            }
        };

        info!(
            stream = %ctx.id(),
            type_url = %response.type_url,
            version = %response.version_info,
            count = response.resources.len(),
            "sending SotW response"
        );

        ctx.record_response();
        Ok(Some(response))
    }

    /// Handle an ACK from the client.
    ///
    /// The client sends an empty `error_detail` to acknowledge.
    pub fn handle_ack(&self, ctx: &StreamContext, type_url: TypeUrl, version: &str, nonce: &str) {
        debug!(
            stream = %ctx.id(),
            type_url = %type_url,
            version = %version,
            nonce = %nonce,
            "received ACK"
        );
    }

    /// Handle a NACK from the client.
    ///
    /// The client sends a populated `error_detail` to reject a response.
    pub fn handle_nack(
        &self,
        ctx: &StreamContext,
        type_url: TypeUrl,
        version: &str,
        nonce: &str,
        error: &str,
    ) {
        warn!(
            stream = %ctx.id(),
            type_url = %type_url,
            version = %version,
            nonce = %nonce,
            error = %error,
            "received NACK"
        );
    }
}

/// Response from the SotW handler.
#[derive(Debug, Clone)]
pub struct SotwResponse {
    /// Version of this response.
    pub version_info: String,
    /// Resources to send.
    pub resources: Vec<xds_core::BoxResource>,
    /// Type URL of the resources.
    pub type_url: TypeUrl,
    /// Unique nonce for this response.
    pub nonce: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use xds_cache::Snapshot;

    fn setup() -> (SotwHandler, NodeHash) {
        let cache = Arc::new(ShardedCache::new());
        let registry = Arc::new(ResourceRegistry::new());
        let handler = SotwHandler::new(cache.clone(), registry);
        let node = NodeHash::from_id("test-node");

        // Set up a snapshot
        let snapshot = Snapshot::builder()
            .version("v1")
            .resources(TypeUrl::CLUSTER.into(), vec![])
            .build();
        cache.set_snapshot(node, snapshot);

        (handler, node)
    }

    #[test]
    fn process_request_no_snapshot() {
        let cache = Arc::new(ShardedCache::new());
        let registry = Arc::new(ResourceRegistry::new());
        let handler = SotwHandler::new(cache, registry);
        let ctx = StreamContext::new();
        let node = NodeHash::from_id("unknown-node");

        let result = handler
            .process_request(&ctx, TypeUrl::CLUSTER.into(), "", &[], node)
            .expect("process_request should not error");
        assert!(result.is_none());
    }

    #[test]
    fn process_request_same_version() {
        let (handler, node) = setup();
        let ctx = StreamContext::new();

        // Request with same version
        let result = handler
            .process_request(&ctx, TypeUrl::CLUSTER.into(), "v1", &[], node)
            .expect("process_request should not error");
        assert!(result.is_none());
    }

    #[test]
    fn process_request_new_version() {
        let (handler, node) = setup();
        let ctx = StreamContext::new();

        // Request with old version
        let result = handler
            .process_request(&ctx, TypeUrl::CLUSTER.into(), "v0", &[], node)
            .expect("process_request should not error");
        assert!(result.is_some());
        let response = result.expect("response should be Some");
        assert_eq!(response.version_info, "v1");
    }

    #[test]
    fn nonce_unique() {
        let n1 = generate_nonce(NoncePrefix::SotW);
        let n2 = generate_nonce(NoncePrefix::SotW);
        assert_ne!(n1, n2);
    }
}
