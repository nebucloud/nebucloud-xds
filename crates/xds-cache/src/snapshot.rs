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
}
