//! Snapshot integration tests.

use nebucloud_xds::prelude::*;

#[test]
fn snapshot_builder_basic() {
    let snapshot = Snapshot::builder().version("v1").build();

    assert_eq!(snapshot.version(), "v1");
    assert!(snapshot.is_empty());
    assert_eq!(snapshot.total_resources(), 0);
}

#[test]
fn snapshot_builder_with_resources() {
    let snapshot = Snapshot::builder()
        .version("v2")
        .resources(TypeUrl::CLUSTER.into(), vec![])
        .resources(TypeUrl::LISTENER.into(), vec![])
        .build();

    assert_eq!(snapshot.version(), "v2");
    assert!(snapshot.contains_type(TypeUrl::CLUSTER.into()));
    assert!(snapshot.contains_type(TypeUrl::LISTENER.into()));
    assert!(!snapshot.contains_type(TypeUrl::ROUTE.into()));
}

#[test]
fn snapshot_type_urls() {
    let snapshot = Snapshot::builder()
        .version("v1")
        .resources(TypeUrl::CLUSTER.into(), vec![])
        .resources(TypeUrl::ENDPOINT.into(), vec![])
        .resources(TypeUrl::LISTENER.into(), vec![])
        .build();

    let type_urls: Vec<_> = snapshot.type_urls().collect();
    assert_eq!(type_urls.len(), 3);
}

#[test]
fn snapshot_version_per_type() {
    let snapshot = Snapshot::builder()
        .version("global-v1")
        .resources(TypeUrl::CLUSTER.into(), vec![])
        .build();

    // Type should inherit global version
    assert_eq!(
        snapshot.get_version(TypeUrl::CLUSTER.into()),
        Some("global-v1")
    );

    // Missing type should return None
    assert_eq!(snapshot.get_version(TypeUrl::LISTENER.into()), None);
}

#[test]
fn snapshot_is_immutable() {
    let snapshot = Snapshot::builder()
        .version("v1")
        .resources(TypeUrl::CLUSTER.into(), vec![])
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
        .resources(TypeUrl::CLUSTER.into(), vec![])
        .build();

    let resources = snapshot.get_resources(TypeUrl::CLUSTER.into()).unwrap();
    assert!(resources.is_empty());
    assert_eq!(resources.len(), 0);
}

#[test]
fn type_url_display() {
    // TypeUrl constants are &'static str
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
}

#[test]
fn type_url_from_string() {
    // TypeUrl can be created from strings using From trait
    let cluster: TypeUrl = "type.googleapis.com/envoy.config.cluster.v3.Cluster".into();
    assert_eq!(cluster.as_str(), TypeUrl::CLUSTER);

    let listener: TypeUrl = TypeUrl::LISTENER.into();
    assert_eq!(
        listener.as_str(),
        "type.googleapis.com/envoy.config.listener.v3.Listener"
    );
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
    // NodeHash displays as 16-character hex string
    assert_eq!(display.len(), 16);
    assert!(display.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn resource_version_equality() {
    // ResourceVersion equality is based on string content
    let v1 = ResourceVersion::new("v1");
    let v2 = ResourceVersion::new("v1");
    let v3 = ResourceVersion::new("v2");

    assert_eq!(v1, v2);
    assert_ne!(v1, v3);
}

#[test]
fn resource_registry_default() {
    let registry = ResourceRegistry::new();
    // Registry should be empty initially
    assert!(registry.is_empty());
}
