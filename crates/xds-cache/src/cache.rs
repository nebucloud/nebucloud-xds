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
        self.snapshots
            .iter()
            .map(|r| (*r.key(), Arc::clone(r.value())))
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
#[allow(dead_code)] // Public API surface
pub struct CacheBuilder {
    capacity: Option<usize>,
    watch_buffer_size: Option<usize>,
}

#[allow(dead_code)] // Public API surface
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
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;

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

    // === Concurrent Access Tests ===

    #[test]
    fn cache_concurrent_reads() {
        let cache = Arc::new(ShardedCache::new());
        let node = NodeHash::from_id("test-node");

        // Pre-populate cache
        cache.set_snapshot(node, Snapshot::builder().version("v1").build());

        let read_count = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        // Spawn 10 reader threads
        for _ in 0..10 {
            let cache = Arc::clone(&cache);
            let count = Arc::clone(&read_count);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    if cache.get_snapshot(node).is_some() {
                        count.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }));
        }

        for handle in handles {
            handle.join().expect("Thread panicked");
        }

        // All reads should succeed
        assert_eq!(read_count.load(Ordering::Relaxed), 1000);
    }

    #[test]
    fn cache_concurrent_writes() {
        let cache = Arc::new(ShardedCache::new());
        let mut handles = vec![];

        // Spawn 10 writer threads, each writing to different nodes
        for i in 0..10 {
            let cache = Arc::clone(&cache);
            handles.push(thread::spawn(move || {
                for j in 0..100 {
                    let node = NodeHash::from_id(&format!("node-{}-{}", i, j));
                    cache
                        .set_snapshot(node, Snapshot::builder().version(format!("v{}", j)).build());
                }
            }));
        }

        for handle in handles {
            handle.join().expect("Thread panicked");
        }

        // All 1000 nodes should be in cache
        assert_eq!(cache.snapshot_count(), 1000);
    }

    #[test]
    fn cache_concurrent_read_write() {
        let cache = Arc::new(ShardedCache::new());
        let node = NodeHash::from_id("contended-node");

        // Pre-populate
        cache.set_snapshot(node, Snapshot::builder().version("v0").build());

        let reads = Arc::new(AtomicUsize::new(0));
        let writes = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        // Writer thread
        {
            let cache = Arc::clone(&cache);
            let writes = Arc::clone(&writes);
            handles.push(thread::spawn(move || {
                for i in 1..=50 {
                    cache
                        .set_snapshot(node, Snapshot::builder().version(format!("v{}", i)).build());
                    writes.fetch_add(1, Ordering::Relaxed);
                    thread::sleep(Duration::from_micros(100));
                }
            }));
        }

        // Reader threads
        for _ in 0..5 {
            let cache = Arc::clone(&cache);
            let reads = Arc::clone(&reads);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    if cache.get_snapshot(node).is_some() {
                        reads.fetch_add(1, Ordering::Relaxed);
                    }
                    thread::sleep(Duration::from_micros(50));
                }
            }));
        }

        for handle in handles {
            handle.join().expect("Thread panicked");
        }

        assert_eq!(writes.load(Ordering::Relaxed), 50);
        // All reads should succeed (snapshot always exists)
        assert_eq!(reads.load(Ordering::Relaxed), 500);
    }

    // === Large Snapshot Tests ===

    #[test]
    fn cache_many_nodes() {
        let cache = ShardedCache::with_capacity(10000);

        // Add 10,000 nodes
        for i in 0..10000 {
            let node = NodeHash::from_id(&format!("node-{}", i));
            cache.set_snapshot(node, Snapshot::builder().version(format!("v{}", i)).build());
        }

        assert_eq!(cache.snapshot_count(), 10000);

        // Verify random access
        for i in [0, 999, 5000, 9999] {
            let node = NodeHash::from_id(&format!("node-{}", i));
            let snap = cache.get_snapshot(node).unwrap();
            assert_eq!(snap.version(), format!("v{}", i));
        }
    }

    #[test]
    fn cache_snapshot_update() {
        let cache = ShardedCache::new();
        let node = NodeHash::from_id("test-node");

        // Initial version
        cache.set_snapshot(node, Snapshot::builder().version("v1").build());
        assert_eq!(cache.get_snapshot(node).unwrap().version(), "v1");

        // Update version
        cache.set_snapshot(node, Snapshot::builder().version("v2").build());
        assert_eq!(cache.get_snapshot(node).unwrap().version(), "v2");

        // Stats should show 2 sets
        assert_eq!(cache.stats().snapshots_set(), 2);
    }

    // === Watch Tests ===

    #[tokio::test]
    async fn cache_multiple_watches_same_node() {
        let cache = ShardedCache::new();
        let node = NodeHash::from_id("test-node");

        let mut watch1 = cache.create_watch(node);
        let mut watch2 = cache.create_watch(node);

        // Set snapshot
        cache.set_snapshot(node, Snapshot::builder().version("v1").build());

        // Both watches should receive it
        let snap1 = watch1.recv().await.unwrap();
        let snap2 = watch2.recv().await.unwrap();
        assert_eq!(snap1.version(), "v1");
        assert_eq!(snap2.version(), "v1");
    }

    #[tokio::test]
    async fn cache_watch_receives_updates() {
        let cache = ShardedCache::new();
        let node = NodeHash::from_id("test-node");

        let mut watch = cache.create_watch(node);

        // Send multiple updates
        for i in 1..=3 {
            cache.set_snapshot(node, Snapshot::builder().version(format!("v{}", i)).build());
        }

        // Watch should receive all updates (buffered)
        let snap1 = watch.recv().await.unwrap();
        assert_eq!(snap1.version(), "v1");

        let snap2 = watch.recv().await.unwrap();
        assert_eq!(snap2.version(), "v2");

        let snap3 = watch.recv().await.unwrap();
        assert_eq!(snap3.version(), "v3");
    }

    // === Edge Cases ===

    #[test]
    fn cache_clear_nonexistent_node() {
        let cache = ShardedCache::new();
        let node = NodeHash::from_id("nonexistent");

        // Should not panic
        cache.clear_snapshot(node);
        assert_eq!(cache.snapshot_count(), 0);
    }

    #[test]
    fn cache_wildcard_node() {
        let cache = ShardedCache::new();
        let wildcard = NodeHash::wildcard();

        cache.set_snapshot(wildcard, Snapshot::builder().version("v1").build());
        assert!(cache.has_snapshot(wildcard));

        let snap = cache.get_snapshot(wildcard).unwrap();
        assert_eq!(snap.version(), "v1");
    }

    #[test]
    fn cache_node_hash_collision_unlikely() {
        // FNV-1a should give different hashes for similar strings
        let node1 = NodeHash::from_id("node-1");
        let node2 = NodeHash::from_id("node-2");
        let node3 = NodeHash::from_id("1-node");

        // All should be different (this is a sanity check, not guaranteed)
        assert_ne!(node1, node2);
        assert_ne!(node2, node3);
        assert_ne!(node1, node3);
    }
}
