//! Delta xDS protocol handler.
//!
//! Delta xDS is an incremental protocol that only sends changed resources
//! rather than the full set on each update.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tracing::{debug, info, trace, warn};
use xds_cache::{Cache, ShardedCache};
use xds_core::{NodeHash, ResourceRegistry, TypeUrl, XdsResult};

use crate::stream::StreamContext;
use crate::utils::{generate_nonce, NoncePrefix};

/// Tracks the state of resources sent to a client.
#[derive(Debug, Default)]
pub struct ClientResourceState {
    /// Resources the client has, keyed by name with version.
    subscribed: HashMap<String, String>,
    /// Resources the client has requested (subscription).
    requested: HashSet<String>,
    /// Whether wildcard subscription is active.
    wildcard: bool,
}

impl ClientResourceState {
    /// Create a new client resource state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Subscribe to specific resources.
    pub fn subscribe(&mut self, names: impl IntoIterator<Item = String>) {
        for name in names {
            self.requested.insert(name);
        }
    }

    /// Unsubscribe from specific resources.
    pub fn unsubscribe(&mut self, names: impl IntoIterator<Item = String>) {
        for name in names {
            self.requested.remove(&name);
            self.subscribed.remove(&name);
        }
    }

    /// Enable wildcard subscription.
    pub fn set_wildcard(&mut self, wildcard: bool) {
        self.wildcard = wildcard;
    }

    /// Check if a resource is subscribed.
    pub fn is_subscribed(&self, name: &str) -> bool {
        self.wildcard || self.requested.contains(name)
    }

    /// Update that a resource was sent to the client.
    pub fn mark_sent(&mut self, name: String, version: String) {
        self.subscribed.insert(name, version);
    }

    /// Get the version of a resource the client has.
    pub fn client_version(&self, name: &str) -> Option<&str> {
        self.subscribed.get(name).map(|s| s.as_str())
    }

    /// Mark resources as removed.
    pub fn mark_removed(&mut self, names: impl IntoIterator<Item = String>) {
        for name in names {
            self.subscribed.remove(&name);
        }
    }
}

/// Handler for Delta xDS requests.
///
/// Delta xDS sends only the resources that have changed since the last
/// response, reducing bandwidth and processing overhead.
#[derive(Debug)]
pub struct DeltaHandler {
    /// Shared cache.
    cache: Arc<ShardedCache>,
    /// Resource registry.
    registry: Arc<ResourceRegistry>,
}

impl DeltaHandler {
    /// Create a new Delta handler.
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

    /// Process an incoming delta discovery request.
    ///
    /// Returns updates (additions/modifications) and removals.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Stream context
    /// * `type_url` - Type of resources
    /// * `client_state` - Current state of what the client has
    /// * `subscribe` - Resources to subscribe to
    /// * `unsubscribe` - Resources to unsubscribe from
    /// * `node_hash` - Node hash
    pub fn process_request(
        &self,
        ctx: &StreamContext,
        type_url: TypeUrl,
        client_state: &mut ClientResourceState,
        subscribe: Vec<String>,
        unsubscribe: Vec<String>,
        node_hash: NodeHash,
    ) -> XdsResult<Option<DeltaResponse>> {
        ctx.record_request();

        trace!(
            stream = %ctx.id(),
            type_url = %type_url,
            subscribe = ?subscribe,
            unsubscribe = ?unsubscribe,
            "processing Delta request"
        );

        // Update subscription state
        if subscribe.is_empty() && unsubscribe.is_empty() && client_state.requested.is_empty() {
            // Initial request with no specific resources = wildcard
            client_state.set_wildcard(true);
        } else {
            client_state.subscribe(subscribe.clone());
            client_state.unsubscribe(unsubscribe.clone());
        }

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

        let mut updated = Vec::new();
        let mut removed = Vec::new();

        // Find updates
        for (name, resource) in resources.iter() {
            if !client_state.is_subscribed(name) {
                continue;
            }

            let version = resources.version();
            let client_version = client_state.client_version(name);

            if client_version != Some(version) {
                updated.push(DeltaResource {
                    name: name.to_string(),
                    version: version.to_string(),
                    resource: resource.clone(),
                });
                client_state.mark_sent(name.to_string(), version.to_string());
            }
        }

        // Find removals (resources client has that no longer exist)
        let current_names: HashSet<&String> = resources.names().collect();
        let removed_names: Vec<String> = client_state
            .subscribed
            .keys()
            .filter(|name| !current_names.contains(name))
            .cloned()
            .collect();

        for name in &removed_names {
            removed.push(name.clone());
        }
        client_state.mark_removed(removed_names);

        if updated.is_empty() && removed.is_empty() {
            return Ok(None);
        }

        let response = DeltaResponse {
            type_url,
            resources: updated,
            removed_resources: removed,
            nonce: generate_nonce(NoncePrefix::Delta),
            system_version_info: snapshot.version().to_string(),
        };

        info!(
            stream = %ctx.id(),
            type_url = %response.type_url,
            updates = response.resources.len(),
            removals = response.removed_resources.len(),
            "sending Delta response"
        );

        ctx.record_response();
        Ok(Some(response))
    }

    /// Handle an ACK from the client.
    pub fn handle_ack(&self, ctx: &StreamContext, type_url: TypeUrl, nonce: &str) {
        debug!(
            stream = %ctx.id(),
            type_url = %type_url,
            nonce = %nonce,
            "received Delta ACK"
        );
    }

    /// Handle a NACK from the client.
    pub fn handle_nack(&self, ctx: &StreamContext, type_url: TypeUrl, nonce: &str, error: &str) {
        warn!(
            stream = %ctx.id(),
            type_url = %type_url,
            nonce = %nonce,
            error = %error,
            "received Delta NACK"
        );
    }
}

/// A resource in a delta response.
#[derive(Debug, Clone)]
pub struct DeltaResource {
    /// Resource name.
    pub name: String,
    /// Resource version.
    pub version: String,
    /// The resource itself.
    pub resource: xds_core::BoxResource,
}

/// Response from the Delta handler.
#[derive(Debug, Clone)]
pub struct DeltaResponse {
    /// Type URL of the resources.
    pub type_url: TypeUrl,
    /// Updated or new resources.
    pub resources: Vec<DeltaResource>,
    /// Names of removed resources.
    pub removed_resources: Vec<String>,
    /// Unique nonce for this response.
    pub nonce: String,
    /// System version info.
    pub system_version_info: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_state_subscribe() {
        let mut state = ClientResourceState::new();
        state.subscribe(["cluster-1".to_string(), "cluster-2".to_string()]);

        assert!(state.is_subscribed("cluster-1"));
        assert!(state.is_subscribed("cluster-2"));
        assert!(!state.is_subscribed("cluster-3"));
    }

    #[test]
    fn client_state_wildcard() {
        let mut state = ClientResourceState::new();
        state.set_wildcard(true);

        assert!(state.is_subscribed("anything"));
        assert!(state.is_subscribed("really-anything"));
    }

    #[test]
    fn client_state_unsubscribe() {
        let mut state = ClientResourceState::new();
        state.subscribe(["cluster-1".to_string()]);
        state.mark_sent("cluster-1".to_string(), "v1".to_string());

        assert!(state.is_subscribed("cluster-1"));
        assert_eq!(state.client_version("cluster-1"), Some("v1"));

        state.unsubscribe(["cluster-1".to_string()]);
        assert!(!state.is_subscribed("cluster-1"));
        assert!(state.client_version("cluster-1").is_none());
    }
}
