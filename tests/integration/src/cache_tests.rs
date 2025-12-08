//! Cache integration tests.

use std::sync::Arc;
use std::time::Duration;

use nebucloud_xds::prelude::*;

#[test]
fn cache_basic_operations() {
    let cache = ShardedCache::new();
    let node = NodeHash::from_id("test-node");

    // Set a snapshot
    let snapshot = Snapshot::builder()
        .version("v1")
        .resources(TypeUrl::Cluster, vec![])
        .build();

    cache.set_snapshot(node, snapshot);

    // Get the snapshot
    let retrieved = cache.get_snapshot(node).expect("snapshot should exist");
    assert_eq!(retrieved.version(), "v1");
    assert!(retrieved.contains_type(TypeUrl::Cluster));
}

#[test]
fn cache_multiple_nodes() {
    let cache = ShardedCache::new();

    // Create different snapshots for different nodes
    let nodes = ["node-1", "node-2", "node-3"];
    for (i, node_id) in nodes.iter().enumerate() {
        let node = NodeHash::from_id(node_id);
        let snapshot = Snapshot::builder()
            .version(format!("v{}", i + 1))
            .build();
        cache.set_snapshot(node, snapshot);
    }

    assert_eq!(cache.snapshot_count(), 3);

    // Verify each node has correct version
    for (i, node_id) in nodes.iter().enumerate() {
        let node = NodeHash::from_id(node_id);
        let snapshot = cache.get_snapshot(node).unwrap();
        assert_eq!(snapshot.version(), format!("v{}", i + 1));
    }
}

#[test]
fn cache_clear_snapshot() {
    let cache = ShardedCache::new();
    let node = NodeHash::from_id("test-node");

    cache.set_snapshot(node, Snapshot::builder().version("v1").build());
    assert!(cache.has_snapshot(node));

    cache.clear_snapshot(node);
    assert!(!cache.has_snapshot(node));
}

#[test]
fn cache_stats_tracking() {
    let cache = ShardedCache::new();
    let node = NodeHash::from_id("test-node");

    // Record miss
    cache.get_snapshot(node);
    assert_eq!(cache.stats().snapshot_misses(), 1);
    assert_eq!(cache.stats().snapshot_hits(), 0);

    // Set and hit
    cache.set_snapshot(node, Snapshot::builder().version("v1").build());
    cache.get_snapshot(node);

    assert_eq!(cache.stats().snapshots_set(), 1);
    assert_eq!(cache.stats().snapshot_hits(), 1);
    assert_eq!(cache.stats().snapshot_misses(), 1);

    // Hit rate should be 0.5
    assert!((cache.stats().hit_rate() - 0.5).abs() < 0.01);
}

#[tokio::test]
async fn cache_watch_notifications() {
    let cache = ShardedCache::new();
    let node = NodeHash::from_id("test-node");

    // Create a watch
    let mut watch = cache.create_watch(node);
    assert_eq!(cache.watches().watch_count(node), 1);

    // Set a snapshot
    cache.set_snapshot(node, Snapshot::builder().version("v1").build());

    // Watch should receive the snapshot
    let result = tokio::time::timeout(Duration::from_secs(1), watch.recv()).await;
    assert!(result.is_ok());
    let snapshot = result.unwrap().unwrap();
    assert_eq!(snapshot.version(), "v1");
}

#[tokio::test]
async fn cache_multiple_watches() {
    let cache = ShardedCache::new();
    let node = NodeHash::from_id("test-node");

    // Create multiple watches
    let mut watch1 = cache.create_watch(node);
    let mut watch2 = cache.create_watch(node);
    assert_eq!(cache.watches().watch_count(node), 2);

    // Set a snapshot
    cache.set_snapshot(node, Snapshot::builder().version("v1").build());

    // Both watches should receive the snapshot
    let r1 = tokio::time::timeout(Duration::from_secs(1), watch1.recv()).await;
    let r2 = tokio::time::timeout(Duration::from_secs(1), watch2.recv()).await;

    assert!(r1.is_ok() && r2.is_ok());
    assert_eq!(r1.unwrap().unwrap().version(), "v1");
    assert_eq!(r2.unwrap().unwrap().version(), "v1");
}

#[test]
fn cache_cancel_watch() {
    let cache = ShardedCache::new();
    let node = NodeHash::from_id("test-node");

    let watch = cache.create_watch(node);
    assert_eq!(cache.watches().watch_count(node), 1);

    cache.cancel_watch(watch.id());
    assert_eq!(cache.watches().watch_count(node), 0);
}

#[test]
fn cache_concurrent_access() {
    use std::thread;

    let cache = Arc::new(ShardedCache::new());
    let mut handles = vec![];

    // Spawn multiple threads doing concurrent operations
    for i in 0..10 {
        let cache_clone = Arc::clone(&cache);
        let handle = thread::spawn(move || {
            let node = NodeHash::from_id(&format!("node-{}", i));

            for j in 0..100 {
                let snapshot = Snapshot::builder()
                    .version(format!("v{}", j))
                    .build();
                cache_clone.set_snapshot(node, snapshot);
                cache_clone.get_snapshot(node);
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Cache should have all nodes
    assert_eq!(cache.snapshot_count(), 10);
}
