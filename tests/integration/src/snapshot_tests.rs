//! Snapshot integration tests.

use nebucloud_xds::prelude::*;

#[test]
fn snapshot_builder_basic() {
    let snapshot = Snapshot::builder()
        .version("v1")
        .build();

    assert_eq!(snapshot.version(), "v1");
    assert!(snapshot.is_empty());
    assert_eq!(snapshot.total_resources(), 0);
}

#[test]
fn snapshot_builder_with_resources() {
    let snapshot = Snapshot::builder()
        .version("v2")
        .resources(TypeUrl::Cluster, vec![])
        .resources(TypeUrl::Listener, vec![])
        .build();

    assert_eq!(snapshot.version(), "v2");
    assert!(snapshot.contains_type(TypeUrl::Cluster));
    assert!(snapshot.contains_type(TypeUrl::Listener));
    assert!(!snapshot.contains_type(TypeUrl::Route));
}

#[test]
fn snapshot_type_urls() {
    let snapshot = Snapshot::builder()
        .version("v1")
        .resources(TypeUrl::Cluster, vec![])
        .resources(TypeUrl::Endpoint, vec![])
        .resources(TypeUrl::Listener, vec![])
        .build();

    let type_urls: Vec<_> = snapshot.type_urls().collect();
    assert_eq!(type_urls.len(), 3);
}

#[test]
fn snapshot_version_per_type() {
    let snapshot = Snapshot::builder()
        .version("global-v1")
        .resources(TypeUrl::Cluster, vec![])
        .build();

    // Type should inherit global version
    assert_eq!(snapshot.get_version(TypeUrl::Cluster), Some("global-v1"));

    // Missing type should return None
    assert_eq!(snapshot.get_version(TypeUrl::Listener), None);
}

#[test]
fn snapshot_is_immutable() {
    let snapshot = Snapshot::builder()
        .version("v1")
        .resources(TypeUrl::Cluster, vec![])
        .build();

    // Store creation time
    let created = snapshot.created_at();

    // Clone and verify
    let cloned = snapshot.clone();
    assert_eq!(cloned.version(), snapshot.version());
    assert_eq!(cloned.created_at(), created);
}

#[test]
fn snapshot_resources_empty() {
    let snapshot = Snapshot::builder()
        .version("v1")
        .resources(TypeUrl::Cluster, vec![])
        .build();

    let resources = snapshot.get_resources(TypeUrl::Cluster).unwrap();
    assert!(resources.is_empty());
    assert_eq!(resources.len(), 0);
}

#[test]
fn type_url_display() {
    assert_eq!(
        TypeUrl::Cluster.as_str(),
        "type.googleapis.com/envoy.config.cluster.v3.Cluster"
    );
    assert_eq!(
        TypeUrl::Listener.as_str(),
        "type.googleapis.com/envoy.config.listener.v3.Listener"
    );
    assert_eq!(
        TypeUrl::Route.as_str(),
        "type.googleapis.com/envoy.config.route.v3.RouteConfiguration"
    );
    assert_eq!(
        TypeUrl::Endpoint.as_str(),
        "type.googleapis.com/envoy.config.endpoint.v3.ClusterLoadAssignment"
    );
}

#[test]
fn type_url_from_str() {
    assert_eq!(
        TypeUrl::from_str("type.googleapis.com/envoy.config.cluster.v3.Cluster"),
        Some(TypeUrl::Cluster)
    );
    assert_eq!(
        TypeUrl::from_str("type.googleapis.com/envoy.config.listener.v3.Listener"),
        Some(TypeUrl::Listener)
    );
    assert_eq!(TypeUrl::from_str("unknown"), None);
}

#[test]
fn node_hash_deterministic() {
    let hash1 = NodeHash::from_id("test-node");
    let hash2 = NodeHash::from_id("test-node");
    let hash3 = NodeHash::from_id("different-node");

    assert_eq!(hash1, hash2);
    assert_ne!(hash1, hash3);
}

#[test]
fn node_hash_display() {
    let hash = NodeHash::from_id("my-node");
    let display = format!("{}", hash);
    assert!(display.starts_with("node-"));
    assert!(display.len() > 5);
}

#[test]
fn resource_version_from_bytes() {
    let data1 = b"some data";
    let data2 = b"some data";
    let data3 = b"different data";

    let v1 = ResourceVersion::from_bytes(data1);
    let v2 = ResourceVersion::from_bytes(data2);
    let v3 = ResourceVersion::from_bytes(data3);

    assert_eq!(v1, v2);
    assert_ne!(v1, v3);
}

#[test]
fn resource_registry_default() {
    let registry = ResourceRegistry::new();
    // Registry should be empty initially
    assert!(registry.is_empty());
}
