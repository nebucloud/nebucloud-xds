//! Cache trait and ShardedCache implementation.
//!
//! The cache stores snapshots keyed by node hash. The [`ShardedCache`]
//! implementation uses `DashMap` for lock-free concurrent access.

use std::sync::Arc;

use dashmap::DashMap;
use tracing::{debug, trace};
use xds_core::NodeHash;

use crate::snapshot::Snapshot;
use crate::stats::CacheStats;
use crate::watch::WatchManager;

/// Trait for xDS snapshot caches.
///
/// Provides the interface for storing and retrieving snapshots.
pub trait Cache: Send + Sync {
    /// Get a snapshot for a node.
    fn get_snapshot(&self, node: NodeHash) -> Option<Arc<Snapshot>>;

    /// Set a snapshot for a node.
    ///
    /// This will notify any watches for this node.
    fn set_snapshot(&self, node: NodeHash, snapshot: Snapshot);

    /// Clear the snapshot for a node.
    fn clear_snapshot(&self, node: NodeHash);

    /// Get the number of cached snapshots.
    fn snapshot_count(&self) -> usize;
}

/// A high-performance sharded cache using DashMap.
///
/// This cache implementation:
/// - Uses `DashMap` for lock-free concurrent reads
/// - Automatically notifies watches on snapshot updates
/// - Tracks statistics for monitoring
///
/// ## Thread Safety
///
/// All operations are thread-safe. The cache uses `DashMap` internally,
/// which provides fine-grained locking at the bucket level rather than
/// a global lock.
///
/// ## Important
///
/// All `DashMap` references are dropped before any async operations
/// to prevent holding locks across await points.
#[derive(Debug)]
pub struct ShardedCache {
    /// Snapshots keyed by node hash.
    snapshots: DashMap<NodeHash, Arc<Snapshot>>,
    /// Watch manager for notifications.
    watches: WatchManager,
    /// Statistics.
    stats: CacheStats,
}

impl Default for ShardedCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ShardedCache {
    /// Create a new sharded cache with default settings.
    pub fn new() -> Self {
        Self::with_capacity(64)
    }

    /// Create a new sharded cache with a specific initial capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            snapshots: DashMap::with_capacity(capacity),
            watches: WatchManager::new(),
            stats: CacheStats::new(),
        }
    }

    /// Get the watch manager for creating watches.
    #[inline]
    pub fn watches(&self) -> &WatchManager {
        &self.watches
    }

    /// Get cache statistics.
    #[inline]
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// Create a watch for a node.
    ///
    /// The watch will receive updates when the snapshot for this node changes.
    /// If a snapshot already exists, the caller should check with `get_snapshot`
    /// first.
    #[inline]
    pub fn create_watch(&self, node: NodeHash) -> crate::watch::Watch {
        self.watches.create_watch(node)
    }

    /// Cancel a watch.
    #[inline]
    pub fn cancel_watch(&self, watch_id: crate::watch::WatchId) {
        self.watches.cancel_watch(watch_id)
    }

    /// Get all node hashes in the cache.
    pub fn nodes(&self) -> Vec<NodeHash> {
        self.snapshots.iter().map(|r| *r.key()).collect()
    }

    /// Check if a snapshot exists for a node.
    pub fn has_snapshot(&self, node: NodeHash) -> bool {
        self.snapshots.contains_key(&node)
    }

    /// Iterate over all snapshots.
    ///
    /// Note: This acquires read locks on all shards.
    pub fn iter(&self) -> impl Iterator<Item = (NodeHash, Arc<Snapshot>)> + '_ {
        self.snapshots.iter().map(|r| (*r.key(), Arc::clone(r.value())))
    }
}

impl Cache for ShardedCache {
    fn get_snapshot(&self, node: NodeHash) -> Option<Arc<Snapshot>> {
        // DashMap::get returns a Ref that holds a read lock.
        // We clone the Arc and drop the Ref immediately.
        let result = self.snapshots.get(&node).map(|r| Arc::clone(&*r));

        if result.is_some() {
            self.stats.record_hit();
            trace!(node = %node, "cache hit");
        } else {
            self.stats.record_miss();
            trace!(node = %node, "cache miss");
        }

        result
    }

    fn set_snapshot(&self, node: NodeHash, snapshot: Snapshot) {
        let snapshot = Arc::new(snapshot);

        // Insert snapshot (DashMap insert is lock-free for the caller)
        self.snapshots.insert(node, Arc::clone(&snapshot));
        self.stats.record_set();

        debug!(
            node = %node,
            version = %snapshot.version(),
            resources = snapshot.total_resources(),
            "set snapshot"
        );

        // Notify watches (no DashMap lock held)
        self.watches.notify(node, snapshot);
    }

    fn clear_snapshot(&self, node: NodeHash) {
        if self.snapshots.remove(&node).is_some() {
            self.stats.record_clear();
            debug!(node = %node, "cleared snapshot");
        }
    }

    fn snapshot_count(&self) -> usize {
        self.snapshots.len()
    }
}

/// Builder for creating a configured cache.
#[derive(Debug, Default)]
pub struct CacheBuilder {
    capacity: Option<usize>,
    watch_buffer_size: Option<usize>,
}

impl CacheBuilder {
    /// Create a new cache builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the initial capacity.
    pub fn capacity(mut self, capacity: usize) -> Self {
        self.capacity = Some(capacity);
        self
    }

    /// Set the watch channel buffer size.
    pub fn watch_buffer_size(mut self, size: usize) -> Self {
        self.watch_buffer_size = Some(size);
        self
    }

    /// Build the cache.
    pub fn build(self) -> ShardedCache {
        let capacity = self.capacity.unwrap_or(64);
        let watch_buffer = self.watch_buffer_size.unwrap_or(16);

        ShardedCache {
            snapshots: DashMap::with_capacity(capacity),
            watches: WatchManager::with_buffer_size(watch_buffer),
            stats: CacheStats::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_basic_operations() {
        let cache = ShardedCache::new();
        let node = NodeHash::from_id("test-node");

        // Initially empty
        assert!(cache.get_snapshot(node).is_none());
        assert_eq!(cache.snapshot_count(), 0);

        // Set a snapshot
        let snapshot = Snapshot::builder().version("v1").build();
        cache.set_snapshot(node, snapshot);

        // Now exists
        assert!(cache.has_snapshot(node));
        assert_eq!(cache.snapshot_count(), 1);

        let retrieved = cache.get_snapshot(node).unwrap();
        assert_eq!(retrieved.version(), "v1");

        // Clear
        cache.clear_snapshot(node);
        assert!(!cache.has_snapshot(node));
        assert_eq!(cache.snapshot_count(), 0);
    }

    #[test]
    fn cache_stats_tracking() {
        let cache = ShardedCache::new();
        let node = NodeHash::from_id("test-node");

        // Miss
        cache.get_snapshot(node);
        assert_eq!(cache.stats().snapshot_misses(), 1);

        // Set
        cache.set_snapshot(node, Snapshot::builder().version("v1").build());
        assert_eq!(cache.stats().snapshots_set(), 1);

        // Hit
        cache.get_snapshot(node);
        assert_eq!(cache.stats().snapshot_hits(), 1);
    }

    #[tokio::test]
    async fn cache_watch_notification() {
        let cache = ShardedCache::new();
        let node = NodeHash::from_id("test-node");

        let mut watch = cache.create_watch(node);

        // Set snapshot
        cache.set_snapshot(node, Snapshot::builder().version("v1").build());

        // Watch should receive it
        let snapshot = watch.recv().await.unwrap();
        assert_eq!(snapshot.version(), "v1");
    }

    #[test]
    fn cache_builder() {
        let cache = CacheBuilder::new()
            .capacity(128)
            .watch_buffer_size(32)
            .build();

        assert_eq!(cache.snapshot_count(), 0);
    }
}
