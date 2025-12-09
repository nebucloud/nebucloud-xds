//! Protocol compliance tests for xDS.
//!
//! These tests verify that our xDS implementation follows the
//! Envoy xDS protocol specification correctly.

#![allow(dead_code)]

use std::sync::Arc;

use xds_cache::{Cache, ShardedCache, Snapshot};
use xds_core::{BoxResource, NodeHash, Resource, ResourceRegistry, TypeUrl};

use crate::services::{AdsConfig, AdsService};
use crate::sotw::SotwHandler;
use crate::stream::StreamContext;

/// Test resource for protocol testing.
#[derive(Debug, Clone)]
struct TestCluster {
    name: String,
    version: String,
}

impl TestCluster {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: "1".to_string(),
        }
    }
}

impl Resource for TestCluster {
    fn type_url(&self) -> &str {
        TypeUrl::CLUSTER
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn encode(&self) -> Result<prost_types::Any, Box<dyn std::error::Error + Send + Sync>> {
        Ok(prost_types::Any {
            type_url: self.type_url().to_string(),
            value: self.name.as_bytes().to_vec(),
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Test resource for listeners.
#[derive(Debug, Clone)]
struct TestListener {
    name: String,
}

impl TestListener {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl Resource for TestListener {
    fn type_url(&self) -> &str {
        TypeUrl::LISTENER
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn encode(&self) -> Result<prost_types::Any, Box<dyn std::error::Error + Send + Sync>> {
        Ok(prost_types::Any {
            type_url: self.type_url().to_string(),
            value: self.name.as_bytes().to_vec(),
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn setup_cache_with_clusters(node_id: &str, clusters: Vec<&str>) -> (Arc<ShardedCache>, NodeHash) {
    let cache = Arc::new(ShardedCache::new());
    let node_hash = NodeHash::from_id(node_id);

    let resources: Vec<BoxResource> = clusters
        .into_iter()
        .map(|name| Arc::new(TestCluster::new(name)) as BoxResource)
        .collect();

    let snapshot = Snapshot::builder()
        .version("v1")
        .resources(TypeUrl::CLUSTER.into(), resources)
        .build();

    cache.set_snapshot(node_hash, snapshot);
    (cache, node_hash)
}

// ============================================================================
// SotW Protocol Tests
// ============================================================================

mod sotw_protocol {
    use super::*;

    /// Per xDS spec: Initial request with empty version_info should trigger response.
    #[test]
    fn initial_request_empty_version() {
        let (cache, node_hash) =
            setup_cache_with_clusters("node-1", vec!["cluster-a", "cluster-b"]);
        let registry = Arc::new(ResourceRegistry::new());
        let handler = SotwHandler::new(cache, registry);
        let ctx = StreamContext::new();

        // Initial request has empty version_info
        let result = handler
            .process_request(&ctx, TypeUrl::CLUSTER.into(), "", &[], node_hash)
            .unwrap();

        assert!(result.is_some(), "Initial request should receive response");
        let response = result.unwrap();
        assert_eq!(response.version_info, "v1");
        assert_eq!(response.resources.len(), 2);
    }

    /// Per xDS spec: Request with current version should not trigger response.
    #[test]
    fn request_with_current_version_no_response() {
        let (cache, node_hash) = setup_cache_with_clusters("node-1", vec!["cluster-a"]);
        let registry = Arc::new(ResourceRegistry::new());
        let handler = SotwHandler::new(cache, registry);
        let ctx = StreamContext::new();

        // Request with same version as cache
        let result = handler
            .process_request(&ctx, TypeUrl::CLUSTER.into(), "v1", &[], node_hash)
            .unwrap();

        assert!(result.is_none(), "Same version should not trigger response");
    }

    /// Per xDS spec: Request with old version should trigger response.
    #[test]
    fn request_with_old_version_gets_response() {
        let (cache, node_hash) = setup_cache_with_clusters("node-1", vec!["cluster-a"]);
        let registry = Arc::new(ResourceRegistry::new());
        let handler = SotwHandler::new(cache, registry);
        let ctx = StreamContext::new();

        // Request with old version
        let result = handler
            .process_request(&ctx, TypeUrl::CLUSTER.into(), "v0", &[], node_hash)
            .unwrap();

        assert!(result.is_some(), "Old version should trigger response");
        assert_eq!(result.unwrap().version_info, "v1");
    }

    /// Per xDS spec: Wildcard subscription (empty resource_names) gets all resources.
    #[test]
    fn wildcard_subscription_returns_all() {
        let (cache, node_hash) =
            setup_cache_with_clusters("node-1", vec!["cluster-a", "cluster-b", "cluster-c"]);
        let registry = Arc::new(ResourceRegistry::new());
        let handler = SotwHandler::new(cache, registry);
        let ctx = StreamContext::new();

        let result = handler
            .process_request(&ctx, TypeUrl::CLUSTER.into(), "", &[], node_hash)
            .unwrap();

        let response = result.unwrap();
        assert_eq!(response.resources.len(), 3);
    }

    /// Per xDS spec: Explicit subscription only returns requested resources.
    #[test]
    fn explicit_subscription_returns_requested() {
        let (cache, node_hash) =
            setup_cache_with_clusters("node-1", vec!["cluster-a", "cluster-b", "cluster-c"]);
        let registry = Arc::new(ResourceRegistry::new());
        let handler = SotwHandler::new(cache, registry);
        let ctx = StreamContext::new();

        // Request only cluster-a and cluster-c
        let result = handler
            .process_request(
                &ctx,
                TypeUrl::CLUSTER.into(),
                "",
                &["cluster-a".to_string(), "cluster-c".to_string()],
                node_hash,
            )
            .unwrap();

        let response = result.unwrap();
        assert_eq!(response.resources.len(), 2);
    }

    /// Per xDS spec: Response nonce must be unique.
    #[test]
    fn response_nonce_is_unique() {
        let (cache, node_hash) = setup_cache_with_clusters("node-1", vec!["cluster-a"]);
        let registry = Arc::new(ResourceRegistry::new());
        let handler = SotwHandler::new(cache, registry);
        let ctx = StreamContext::new();

        let r1 = handler
            .process_request(&ctx, TypeUrl::CLUSTER.into(), "", &[], node_hash)
            .unwrap()
            .unwrap();

        // Change version to force new response
        let snapshot = Snapshot::builder()
            .version("v2")
            .resources(
                TypeUrl::CLUSTER.into(),
                vec![Arc::new(TestCluster::new("cluster-a")) as BoxResource],
            )
            .build();
        handler.cache().set_snapshot(node_hash, snapshot);

        let r2 = handler
            .process_request(&ctx, TypeUrl::CLUSTER.into(), "v1", &[], node_hash)
            .unwrap()
            .unwrap();

        assert_ne!(r1.nonce, r2.nonce, "Each response should have unique nonce");
    }

    /// Per xDS spec: Unknown node should not receive resources.
    #[test]
    fn unknown_node_no_response() {
        let cache = Arc::new(ShardedCache::new());
        let registry = Arc::new(ResourceRegistry::new());
        let handler = SotwHandler::new(cache, registry);
        let ctx = StreamContext::new();

        let unknown_node = NodeHash::from_id("unknown-node");
        let result = handler
            .process_request(&ctx, TypeUrl::CLUSTER.into(), "", &[], unknown_node)
            .unwrap();

        assert!(result.is_none(), "Unknown node should not receive response");
    }

    /// Per xDS spec: Response type_url must match request.
    #[test]
    fn response_type_url_matches_request() {
        let (cache, node_hash) = setup_cache_with_clusters("node-1", vec!["cluster-a"]);
        let registry = Arc::new(ResourceRegistry::new());
        let handler = SotwHandler::new(cache, registry);
        let ctx = StreamContext::new();

        let result = handler
            .process_request(&ctx, TypeUrl::CLUSTER.into(), "", &[], node_hash)
            .unwrap()
            .unwrap();

        assert_eq!(result.type_url.as_str(), TypeUrl::CLUSTER);
    }
}

// ============================================================================
// ADS Protocol Tests
// ============================================================================

mod ads_protocol {
    use super::*;

    /// ADS should handle multiple resource types.
    #[test]
    fn ads_handles_multiple_types() {
        let cache = Arc::new(ShardedCache::new());
        let registry = Arc::new(ResourceRegistry::new());
        let service = AdsService::new(cache.clone(), registry);

        let node_hash = NodeHash::from_id("node-1");

        // Add clusters
        let cluster: BoxResource = Arc::new(TestCluster::new("cluster-1"));
        let listener: BoxResource = Arc::new(TestListener::new("listener-1"));

        let snapshot = Snapshot::builder()
            .version("v1")
            .resources(TypeUrl::CLUSTER.into(), vec![cluster])
            .resources(TypeUrl::LISTENER.into(), vec![listener])
            .build();

        cache.set_snapshot(node_hash, snapshot);

        let ctx = StreamContext::new();

        // Request clusters
        let cluster_response = service
            .process_sotw_request(&ctx, TypeUrl::CLUSTER, "", &[], node_hash, "", None)
            .unwrap();
        assert!(cluster_response.is_some());

        // Request listeners
        let listener_response = service
            .process_sotw_request(&ctx, TypeUrl::LISTENER, "", &[], node_hash, "", None)
            .unwrap();
        assert!(listener_response.is_some());
    }

    /// ADS config customization.
    #[test]
    fn ads_config_customization() {
        let cache = Arc::new(ShardedCache::new());
        let registry = Arc::new(ResourceRegistry::new());
        let config = AdsConfig {
            max_concurrent_streams: 50,
            response_buffer_size: 8,
            enable_delta: false,
        };

        let service = AdsService::with_config(cache, registry, config);
        assert!(!service.config().enable_delta);
        assert_eq!(service.config().max_concurrent_streams, 50);
    }
}

// ============================================================================
// Resource Type Tests
// ============================================================================

mod resource_types {
    use super::*;

    /// Verify all standard xDS type URLs.
    #[test]
    fn standard_type_urls() {
        assert_eq!(
            TypeUrl::CLUSTER,
            "type.googleapis.com/envoy.config.cluster.v3.Cluster"
        );
        assert_eq!(
            TypeUrl::LISTENER,
            "type.googleapis.com/envoy.config.listener.v3.Listener"
        );
        assert_eq!(
            TypeUrl::ROUTE,
            "type.googleapis.com/envoy.config.route.v3.RouteConfiguration"
        );
        assert_eq!(
            TypeUrl::ENDPOINT,
            "type.googleapis.com/envoy.config.endpoint.v3.ClusterLoadAssignment"
        );
        assert_eq!(
            TypeUrl::SECRET,
            "type.googleapis.com/envoy.extensions.transport_sockets.tls.v3.Secret"
        );
    }

    /// TypeUrl parsing and creation.
    #[test]
    fn type_url_parsing() {
        let cluster_url = TypeUrl::new(TypeUrl::CLUSTER);
        assert_eq!(cluster_url.as_str(), TypeUrl::CLUSTER);

        let custom_url = TypeUrl::new("custom.type/Resource");
        assert_eq!(custom_url.as_str(), "custom.type/Resource");
    }
}

// ============================================================================
// Node Identification Tests
// ============================================================================

mod node_identification {
    use super::*;

    /// Different nodes should have different hashes.
    #[test]
    fn different_nodes_different_hashes() {
        let hash1 = NodeHash::from_id("node-1");
        let hash2 = NodeHash::from_id("node-2");
        assert_ne!(hash1, hash2);
    }

    /// Same node ID should produce same hash.
    #[test]
    fn same_node_same_hash() {
        let hash1 = NodeHash::from_id("node-1");
        let hash2 = NodeHash::from_id("node-1");
        assert_eq!(hash1, hash2);
    }

    /// Each node should have isolated snapshot.
    #[test]
    fn node_isolation() {
        let cache = Arc::new(ShardedCache::new());
        let registry = Arc::new(ResourceRegistry::new());
        let handler = SotwHandler::new(cache.clone(), registry);

        let node1 = NodeHash::from_id("node-1");
        let node2 = NodeHash::from_id("node-2");

        // Set different snapshots for different nodes
        let snapshot1 = Snapshot::builder()
            .version("v1-node1")
            .resources(
                TypeUrl::CLUSTER.into(),
                vec![Arc::new(TestCluster::new("cluster-for-node1")) as BoxResource],
            )
            .build();

        let snapshot2 = Snapshot::builder()
            .version("v1-node2")
            .resources(
                TypeUrl::CLUSTER.into(),
                vec![Arc::new(TestCluster::new("cluster-for-node2")) as BoxResource],
            )
            .build();

        cache.set_snapshot(node1, snapshot1);
        cache.set_snapshot(node2, snapshot2);

        let ctx = StreamContext::new();

        let r1 = handler
            .process_request(&ctx, TypeUrl::CLUSTER.into(), "", &[], node1)
            .unwrap()
            .unwrap();
        let r2 = handler
            .process_request(&ctx, TypeUrl::CLUSTER.into(), "", &[], node2)
            .unwrap()
            .unwrap();

        assert_eq!(r1.version_info, "v1-node1");
        assert_eq!(r2.version_info, "v1-node2");
    }
}

// ============================================================================
// Versioning Tests
// ============================================================================

mod versioning {
    use super::*;

    /// Version comparison should be string-based.
    #[test]
    fn version_string_comparison() {
        let (cache, node_hash) = setup_cache_with_clusters("node-1", vec!["cluster-a"]);
        let registry = Arc::new(ResourceRegistry::new());
        let handler = SotwHandler::new(cache.clone(), registry);
        let ctx = StreamContext::new();

        // Get initial response
        let r1 = handler
            .process_request(&ctx, TypeUrl::CLUSTER.into(), "", &[], node_hash)
            .unwrap()
            .unwrap();

        // Request with exact same version string
        let r2 = handler
            .process_request(
                &ctx,
                TypeUrl::CLUSTER.into(),
                &r1.version_info,
                &[],
                node_hash,
            )
            .unwrap();

        assert!(
            r2.is_none(),
            "Exact version match should not trigger response"
        );
    }

    /// Semantic version strings work correctly.
    #[test]
    fn semantic_versions() {
        let cache = Arc::new(ShardedCache::new());
        let registry = Arc::new(ResourceRegistry::new());
        let handler = SotwHandler::new(cache.clone(), registry);
        let ctx = StreamContext::new();
        let node_hash = NodeHash::from_id("node-1");

        // Set snapshot with semantic version
        let snapshot = Snapshot::builder()
            .version("1.2.3")
            .resources(
                TypeUrl::CLUSTER.into(),
                vec![Arc::new(TestCluster::new("cluster-a")) as BoxResource],
            )
            .build();
        cache.set_snapshot(node_hash, snapshot);

        let response = handler
            .process_request(&ctx, TypeUrl::CLUSTER.into(), "1.2.2", &[], node_hash)
            .unwrap()
            .unwrap();

        assert_eq!(response.version_info, "1.2.3");
    }
}
