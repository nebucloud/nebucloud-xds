//! Snapshot: immutable collection of xDS resources.
//!
//! A snapshot represents a consistent view of all resources for a node
//! at a specific version. Snapshots are:
//!
//! - **Immutable**: Once created, a snapshot cannot be modified
//! - **Versioned**: Each snapshot has a version string
//! - **Type-organized**: Resources are grouped by their type URL

use std::collections::HashMap;
use std::sync::Arc;

use xds_core::{BoxResource, TypeUrl};

/// Resources for a specific type within a snapshot.
#[derive(Debug, Clone, Default)]
pub struct SnapshotResources {
    /// Version string for this resource type.
    version: String,
    /// Resources keyed by name.
    resources: HashMap<String, BoxResource>,
}

impl SnapshotResources {
    /// Create a new empty resource collection.
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            version: version.into(),
            resources: HashMap::new(),
        }
    }

    /// Get the version for this resource type.
    #[inline]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Get the number of resources.
    #[inline]
    pub fn len(&self) -> usize {
        self.resources.len()
    }

    /// Check if there are no resources.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }

    /// Get a resource by name.
    #[inline]
    pub fn get(&self, name: &str) -> Option<&BoxResource> {
        self.resources.get(name)
    }

    /// Iterate over all resources.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&String, &BoxResource)> {
        self.resources.iter()
    }

    /// Get all resource names.
    #[inline]
    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.resources.keys()
    }

    /// Get all resources as a vec.
    pub fn to_vec(&self) -> Vec<BoxResource> {
        self.resources.values().cloned().collect()
    }
}

/// An immutable snapshot of xDS resources for a node.
///
/// Snapshots are the primary unit of cache storage. Each snapshot
/// contains resources organized by type, with per-type versioning.
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// Global version for this snapshot.
    version: String,
    /// Resources grouped by type URL.
    resources: HashMap<TypeUrl, SnapshotResources>,
    /// Creation timestamp.
    created_at: std::time::Instant,
}

impl Snapshot {
    /// Create a new snapshot builder.
    pub fn builder() -> SnapshotBuilder {
        SnapshotBuilder::new()
    }

    /// Get the global version of this snapshot.
    #[inline]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Get the creation timestamp.
    #[inline]
    pub fn created_at(&self) -> std::time::Instant {
        self.created_at
    }

    /// Get resources for a specific type.
    #[inline]
    pub fn get_resources(&self, type_url: TypeUrl) -> Option<&SnapshotResources> {
        self.resources.get(&type_url)
    }

    /// Get the version for a specific resource type.
    #[inline]
    pub fn get_version(&self, type_url: TypeUrl) -> Option<&str> {
        self.resources.get(&type_url).map(|r| r.version.as_str())
    }

    /// Check if this snapshot contains a specific resource type.
    #[inline]
    pub fn contains_type(&self, type_url: TypeUrl) -> bool {
        self.resources.contains_key(&type_url)
    }

    /// Get all type URLs present in this snapshot.
    pub fn type_urls(&self) -> impl Iterator<Item = &TypeUrl> {
        self.resources.keys()
    }

    /// Get the total number of resources across all types.
    pub fn total_resources(&self) -> usize {
        self.resources.values().map(|r| r.len()).sum()
    }

    /// Check if this snapshot is empty (no resources).
    pub fn is_empty(&self) -> bool {
        self.resources.is_empty() || self.resources.values().all(|r| r.is_empty())
    }
}

/// Builder for creating snapshots.
#[derive(Debug, Default)]
pub struct SnapshotBuilder {
    version: String,
    resources: HashMap<TypeUrl, SnapshotResources>,
}

impl SnapshotBuilder {
    /// Create a new snapshot builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the global version for this snapshot.
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Add resources of a specific type.
    ///
    /// The version for this resource type defaults to the global version.
    pub fn resources(
        self,
        type_url: TypeUrl,
        resources: impl IntoIterator<Item = BoxResource>,
    ) -> Self {
        let version = self.version.clone();
        self.resources_with_version(type_url, version, resources)
    }

    /// Add resources of a specific type with a custom version.
    pub fn resources_with_version(
        mut self,
        type_url: TypeUrl,
        version: impl Into<String>,
        resources: impl IntoIterator<Item = BoxResource>,
    ) -> Self {
        let mut snapshot_resources = SnapshotResources::new(version);
        for resource in resources {
            snapshot_resources
                .resources
                .insert(resource.name().to_string(), resource);
        }
        self.resources.insert(type_url, snapshot_resources);
        self
    }

    /// Add a single resource.
    pub fn resource(mut self, type_url: TypeUrl, resource: BoxResource) -> Self {
        let entry = self
            .resources
            .entry(type_url)
            .or_insert_with(|| SnapshotResources::new(self.version.clone()));
        entry
            .resources
            .insert(resource.name().to_string(), resource);
        self
    }

    /// Build the snapshot.
    pub fn build(self) -> Snapshot {
        Snapshot {
            version: self.version,
            resources: self.resources,
            created_at: std::time::Instant::now(),
        }
    }
}

/// Wrapper around `Arc<Snapshot>` for convenient sharing.
#[allow(dead_code)] // Public API surface
pub type SharedSnapshot = Arc<Snapshot>;

#[cfg(test)]
mod tests {
    use super::*;
    use xds_core::Resource;

    /// A simple test resource for testing
    #[derive(Debug)]
    struct TestResource {
        name: String,
        data: Vec<u8>,
    }

    impl TestResource {
        fn new(name: impl Into<String>) -> Self {
            Self {
                name: name.into(),
                data: vec![],
            }
        }
    }

    impl Resource for TestResource {
        fn type_url(&self) -> &str {
            "test.type/TestResource"
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn encode(&self) -> Result<prost_types::Any, Box<dyn std::error::Error + Send + Sync>> {
            Ok(prost_types::Any {
                type_url: self.type_url().to_string(),
                value: self.data.clone(),
            })
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    #[test]
    fn snapshot_builder_basic() {
        let snapshot = Snapshot::builder().version("v1").build();

        assert_eq!(snapshot.version(), "v1");
        assert!(snapshot.is_empty());
    }

    #[test]
    fn snapshot_builder_with_resources() {
        let snapshot = Snapshot::builder()
            .version("v2")
            .resources(TypeUrl::CLUSTER.into(), vec![])
            .build();

        assert_eq!(snapshot.version(), "v2");
        assert!(snapshot.contains_type(TypeUrl::CLUSTER.into()));
    }

    #[test]
    fn snapshot_resources_version() {
        let snapshot = Snapshot::builder()
            .version("global-v1")
            .resources(TypeUrl::CLUSTER.into(), vec![])
            .build();

        assert_eq!(
            snapshot.get_version(TypeUrl::CLUSTER.into()),
            Some("global-v1")
        );
    }

    #[test]
    fn snapshot_with_custom_version_per_type() {
        let snapshot = Snapshot::builder()
            .version("global-v1")
            .resources_with_version(TypeUrl::CLUSTER.into(), "cluster-v2", vec![])
            .resources_with_version(TypeUrl::LISTENER.into(), "listener-v3", vec![])
            .build();

        assert_eq!(
            snapshot.get_version(TypeUrl::CLUSTER.into()),
            Some("cluster-v2")
        );
        assert_eq!(
            snapshot.get_version(TypeUrl::LISTENER.into()),
            Some("listener-v3")
        );
    }

    #[test]
    fn snapshot_with_actual_resources() {
        let resource1: BoxResource = Arc::new(TestResource::new("resource-1"));
        let resource2: BoxResource = Arc::new(TestResource::new("resource-2"));

        let type_url = TypeUrl::new("test.type/TestResource");

        let snapshot = Snapshot::builder()
            .version("v1")
            .resources(type_url.clone(), vec![resource1, resource2])
            .build();

        assert_eq!(snapshot.total_resources(), 2);
        assert!(!snapshot.is_empty());

        let resources = snapshot.get_resources(type_url).unwrap();
        assert_eq!(resources.len(), 2);
    }

    #[test]
    fn snapshot_get_resource_by_name() {
        let resource: BoxResource = Arc::new(TestResource::new("my-resource"));
        let type_url = TypeUrl::new("test.type/TestResource");

        let snapshot = Snapshot::builder()
            .version("v1")
            .resources(type_url.clone(), vec![resource])
            .build();

        // Use get_resources to get the SnapshotResources, then get() to find by name
        let resources = snapshot.get_resources(type_url.clone()).unwrap();
        let found = resources.get("my-resource");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "my-resource");

        let not_found = resources.get("nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn snapshot_add_single_resource() {
        let resource: BoxResource = Arc::new(TestResource::new("single"));
        let type_url = TypeUrl::new("test.type/TestResource");

        let snapshot = Snapshot::builder()
            .version("v1")
            .resource(type_url.clone(), resource)
            .build();

        assert_eq!(snapshot.total_resources(), 1);
    }

    #[test]
    fn snapshot_multiple_types() {
        let cluster: BoxResource = Arc::new(TestResource::new("cluster-1"));
        let listener: BoxResource = Arc::new(TestResource::new("listener-1"));
        let route: BoxResource = Arc::new(TestResource::new("route-1"));

        let snapshot = Snapshot::builder()
            .version("v1")
            .resource(TypeUrl::CLUSTER.into(), cluster)
            .resource(TypeUrl::LISTENER.into(), listener)
            .resource(TypeUrl::ROUTE.into(), route)
            .build();

        assert_eq!(snapshot.total_resources(), 3);
        assert!(snapshot.contains_type(TypeUrl::CLUSTER.into()));
        assert!(snapshot.contains_type(TypeUrl::LISTENER.into()));
        assert!(snapshot.contains_type(TypeUrl::ROUTE.into()));
    }

    #[test]
    fn snapshot_type_urls() {
        let cluster: BoxResource = Arc::new(TestResource::new("cluster-1"));
        let listener: BoxResource = Arc::new(TestResource::new("listener-1"));

        let snapshot = Snapshot::builder()
            .version("v1")
            .resource(TypeUrl::CLUSTER.into(), cluster)
            .resource(TypeUrl::LISTENER.into(), listener)
            .build();

        let type_urls: Vec<_> = snapshot.type_urls().collect();
        assert_eq!(type_urls.len(), 2);
    }

    #[test]
    fn snapshot_created_at() {
        use std::time::Instant;

        let before = Instant::now();
        let snapshot = Snapshot::builder().version("v1").build();
        let after = Instant::now();

        assert!(snapshot.created_at() >= before);
        assert!(snapshot.created_at() <= after);
    }

    #[test]
    fn snapshot_age_via_created_at() {
        use std::time::Duration;

        let snapshot = Snapshot::builder().version("v1").build();
        std::thread::sleep(Duration::from_millis(10));

        // Calculate age from created_at
        let elapsed = snapshot.created_at().elapsed();
        assert!(elapsed.as_millis() >= 10);
    }

    #[test]
    fn snapshot_resource_iteration() {
        let resource: BoxResource = Arc::new(TestResource::new("my-resource"));
        let type_url = TypeUrl::new("test.type/TestResource");

        let snapshot = Snapshot::builder()
            .version("v1")
            .resources(type_url.clone(), vec![resource])
            .build();

        assert_eq!(snapshot.total_resources(), 1);
        let resources = snapshot.get_resources(type_url).unwrap();
        // Use the iter() method instead of indexing
        let items: Vec<_> = resources.iter().collect();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].1.name(), "my-resource");
    }

    #[test]
    fn snapshot_empty_version() {
        let snapshot = Snapshot::builder().version("").build();
        assert_eq!(snapshot.version(), "");
    }

    #[test]
    fn snapshot_is_empty_with_empty_resources() {
        // A snapshot with type but no actual resources
        let snapshot = Snapshot::builder()
            .version("v1")
            .resources(TypeUrl::CLUSTER.into(), vec![])
            .build();

        // is_empty should return true since there are no actual resources
        assert!(snapshot.is_empty());
    }
}
