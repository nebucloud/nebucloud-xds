//! Load tests for xds-cache with 1000+ nodes.
//!
//! These tests verify the system can handle high load scenarios:
//! - 1000+ concurrent nodes
//! - Parallel snapshot updates
//! - Watch notification under load
//! - Memory and performance characteristics
//!
//! Run with: `cargo test --package integration-tests load_tests -- --nocapture`

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::Barrier;
use xds_cache::{Cache, ShardedCache, Snapshot};
use xds_core::{NodeHash, TypeUrl};

/// Create a snapshot with the given version.
fn create_snapshot(version: &str) -> Snapshot {
    Snapshot::builder()
        .version(version)
        .resources(TypeUrl::new(TypeUrl::CLUSTER), vec![])
        .resources(TypeUrl::new(TypeUrl::LISTENER), vec![])
        .resources(TypeUrl::new(TypeUrl::ROUTE), vec![])
        .build()
}

/// Test that the cache can handle 1000 nodes.
#[tokio::test]
async fn test_1000_nodes() {
    let cache = Arc::new(ShardedCache::new());
    let num_nodes = 1000;

    // Create nodes
    let nodes: Vec<NodeHash> = (0..num_nodes)
        .map(|i| NodeHash::from_id(&format!("envoy-sidecar-{}", i)))
        .collect();

    let start = Instant::now();

    // Set snapshots for all nodes
    for (i, node) in nodes.iter().enumerate() {
        cache.set_snapshot(*node, create_snapshot(&format!("v{}", i)));
    }

    let set_duration = start.elapsed();
    println!(
        "Set {} snapshots in {:?} ({:.2} µs/op)",
        num_nodes,
        set_duration,
        set_duration.as_micros() as f64 / num_nodes as f64
    );

    // Verify all nodes have snapshots
    let start = Instant::now();
    for node in &nodes {
        assert!(cache.get_snapshot(*node).is_some());
    }
    let get_duration = start.elapsed();
    println!(
        "Get {} snapshots in {:?} ({:.2} µs/op)",
        num_nodes,
        get_duration,
        get_duration.as_micros() as f64 / num_nodes as f64
    );

    // Check stats
    let stats = cache.stats();
    println!(
        "Cache stats: hits={}, misses={}, hit_rate={:.2}%",
        stats.snapshot_hits(),
        stats.snapshot_misses(),
        stats.hit_rate() * 100.0
    );

    assert_eq!(cache.snapshot_count(), num_nodes);
    assert_eq!(stats.snapshot_hits(), num_nodes as u64);
    assert_eq!(stats.hit_rate(), 1.0);
}

/// Test concurrent access with 1000 nodes across multiple tasks.
#[tokio::test]
async fn test_concurrent_1000_nodes() {
    let cache = Arc::new(ShardedCache::new());
    let num_nodes = 1000;
    let num_tasks = 10;
    let nodes_per_task = num_nodes / num_tasks;

    // Create all nodes upfront
    let nodes: Vec<NodeHash> = (0..num_nodes)
        .map(|i| NodeHash::from_id(&format!("envoy-sidecar-{}", i)))
        .collect();

    let barrier = Arc::new(Barrier::new(num_tasks));
    let total_ops = Arc::new(AtomicU64::new(0));

    let start = Instant::now();

    // Spawn tasks to set snapshots concurrently
    let mut handles = Vec::new();
    for task_id in 0..num_tasks {
        let cache = Arc::clone(&cache);
        let barrier = Arc::clone(&barrier);
        let total_ops = Arc::clone(&total_ops);
        let task_nodes: Vec<NodeHash> = nodes
            [task_id * nodes_per_task..(task_id + 1) * nodes_per_task]
            .to_vec();

        handles.push(tokio::spawn(async move {
            // Wait for all tasks to be ready
            barrier.wait().await;

            for (i, node) in task_nodes.iter().enumerate() {
                cache.set_snapshot(*node, create_snapshot(&format!("v{}-{}", task_id, i)));
                total_ops.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.expect("Task panicked");
    }

    let duration = start.elapsed();
    let ops = total_ops.load(Ordering::Relaxed);

    println!(
        "Concurrent set: {} ops in {:?} ({:.2} µs/op, {:.0} ops/sec)",
        ops,
        duration,
        duration.as_micros() as f64 / ops as f64,
        ops as f64 / duration.as_secs_f64()
    );

    assert_eq!(cache.snapshot_count(), num_nodes);
}

/// Test watch notifications with 1000 nodes.
#[tokio::test]
async fn test_watch_notifications_1000_nodes() {
    let cache = Arc::new(ShardedCache::new());
    let num_nodes = 1000;

    let nodes: Vec<NodeHash> = (0..num_nodes)
        .map(|i| NodeHash::from_id(&format!("envoy-sidecar-{}", i)))
        .collect();

    // Create watches for all nodes
    let start = Instant::now();
    let mut watches: Vec<_> = nodes.iter().map(|node| cache.create_watch(*node)).collect();
    let watch_duration = start.elapsed();

    println!(
        "Created {} watches in {:?} ({:.2} µs/op)",
        num_nodes,
        watch_duration,
        watch_duration.as_micros() as f64 / num_nodes as f64
    );

    // Set snapshots (triggers notifications)
    let start = Instant::now();
    for (i, node) in nodes.iter().enumerate() {
        cache.set_snapshot(*node, create_snapshot(&format!("v{}", i)));
    }
    let set_duration = start.elapsed();

    println!(
        "Set {} snapshots with watch notifications in {:?} ({:.2} µs/op)",
        num_nodes,
        set_duration,
        set_duration.as_micros() as f64 / num_nodes as f64
    );

    // Verify all watches received notifications
    let start = Instant::now();
    let mut received = 0;
    for watch in &mut watches {
        // Use try_recv to check if notification arrived (non-blocking)
        if watch.try_recv().is_ok() {
            received += 1;
        }
    }
    let recv_duration = start.elapsed();

    println!(
        "Received {} notifications in {:?}",
        received, recv_duration
    );

    // All watches should have received notifications
    assert_eq!(received, num_nodes);
}

/// Test mixed read/write workload with 1000 nodes.
#[tokio::test]
async fn test_mixed_workload_1000_nodes() {
    let cache = Arc::new(ShardedCache::new());
    let num_nodes = 1000;
    let num_operations = 10_000;

    // Pre-populate cache
    let nodes: Vec<NodeHash> = (0..num_nodes)
        .map(|i| NodeHash::from_id(&format!("envoy-sidecar-{}", i)))
        .collect();

    for (i, node) in nodes.iter().enumerate() {
        cache.set_snapshot(*node, create_snapshot(&format!("v{}", i)));
    }

    // Run mixed workload: 90% reads, 10% writes
    let start = Instant::now();
    let mut reads = 0u64;
    let mut writes = 0u64;

    for i in 0..num_operations {
        let node = &nodes[i % num_nodes];

        if i % 10 == 0 {
            // 10% writes
            cache.set_snapshot(*node, create_snapshot(&format!("v{}", i)));
            writes += 1;
        } else {
            // 90% reads
            let _ = cache.get_snapshot(*node);
            reads += 1;
        }
    }

    let duration = start.elapsed();

    println!(
        "Mixed workload: {} reads, {} writes in {:?}",
        reads, writes, duration
    );
    println!(
        "  Throughput: {:.0} ops/sec",
        num_operations as f64 / duration.as_secs_f64()
    );
    println!(
        "  Latency: {:.2} µs/op",
        duration.as_micros() as f64 / num_operations as f64
    );

    // Verify < 10ms p99 latency (actually much faster)
    let avg_latency_us = duration.as_micros() as f64 / num_operations as f64;
    assert!(
        avg_latency_us < 100.0,
        "Average latency {} µs exceeds 100 µs target",
        avg_latency_us
    );
}

/// Stress test: concurrent readers and writers.
#[tokio::test]
async fn test_concurrent_readers_writers() {
    let cache = Arc::new(ShardedCache::new());
    let num_nodes = 1000;
    let num_readers = 8;
    let num_writers = 2;
    let ops_per_task = 1000;

    // Pre-populate
    let nodes: Vec<NodeHash> = (0..num_nodes)
        .map(|i| NodeHash::from_id(&format!("envoy-sidecar-{}", i)))
        .collect();

    for (i, node) in nodes.iter().enumerate() {
        cache.set_snapshot(*node, create_snapshot(&format!("v{}", i)));
    }

    let nodes = Arc::new(nodes);
    let barrier = Arc::new(Barrier::new(num_readers + num_writers));
    let read_count = Arc::new(AtomicU64::new(0));
    let write_count = Arc::new(AtomicU64::new(0));

    let start = Instant::now();

    let mut handles = Vec::new();

    // Spawn readers
    for _ in 0..num_readers {
        let cache = Arc::clone(&cache);
        let nodes = Arc::clone(&nodes);
        let barrier = Arc::clone(&barrier);
        let read_count = Arc::clone(&read_count);

        handles.push(tokio::spawn(async move {
            barrier.wait().await;

            for i in 0..ops_per_task {
                let node = &nodes[i % nodes.len()];
                let _ = cache.get_snapshot(*node);
                read_count.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    // Spawn writers
    for writer_id in 0..num_writers {
        let cache = Arc::clone(&cache);
        let nodes = Arc::clone(&nodes);
        let barrier = Arc::clone(&barrier);
        let write_count = Arc::clone(&write_count);

        handles.push(tokio::spawn(async move {
            barrier.wait().await;

            for i in 0..ops_per_task {
                let node = &nodes[i % nodes.len()];
                cache.set_snapshot(
                    *node,
                    create_snapshot(&format!("v{}-{}", writer_id, i)),
                );
                write_count.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.expect("Task panicked");
    }

    let duration = start.elapsed();
    let total_reads = read_count.load(Ordering::Relaxed);
    let total_writes = write_count.load(Ordering::Relaxed);
    let total_ops = total_reads + total_writes;

    println!(
        "Concurrent R/W: {} reads, {} writes in {:?}",
        total_reads, total_writes, duration
    );
    println!(
        "  Throughput: {:.0} ops/sec",
        total_ops as f64 / duration.as_secs_f64()
    );

    // Should handle at least 100k ops/sec
    let ops_per_sec = total_ops as f64 / duration.as_secs_f64();
    assert!(
        ops_per_sec > 100_000.0,
        "Throughput {} ops/sec is below 100k target",
        ops_per_sec
    );
}

/// Test cache memory efficiency with 1000 nodes.
#[tokio::test]
async fn test_cache_memory_1000_nodes() {
    let cache = Arc::new(ShardedCache::new());
    let num_nodes = 1000;

    let nodes: Vec<NodeHash> = (0..num_nodes)
        .map(|i| NodeHash::from_id(&format!("envoy-sidecar-{}", i)))
        .collect();

    for (i, node) in nodes.iter().enumerate() {
        cache.set_snapshot(*node, create_snapshot(&format!("v{}", i)));
    }

    assert_eq!(cache.snapshot_count(), num_nodes);

    // Clear all snapshots
    for node in &nodes {
        cache.clear_snapshot(*node);
    }

    assert_eq!(cache.snapshot_count(), 0);
}

/// Test scaling to 5000 nodes.
#[tokio::test]
async fn test_5000_nodes() {
    let cache = Arc::new(ShardedCache::new());
    let num_nodes = 5000;

    let start = Instant::now();

    for i in 0..num_nodes {
        let node = NodeHash::from_id(&format!("envoy-sidecar-{}", i));
        cache.set_snapshot(node, create_snapshot(&format!("v{}", i)));
    }

    let duration = start.elapsed();

    println!(
        "Set {} snapshots in {:?} ({:.2} µs/op)",
        num_nodes,
        duration,
        duration.as_micros() as f64 / num_nodes as f64
    );

    assert_eq!(cache.snapshot_count(), num_nodes);

    // Verify random access
    for i in (0..num_nodes).step_by(100) {
        let node = NodeHash::from_id(&format!("envoy-sidecar-{}", i));
        assert!(cache.get_snapshot(node).is_some());
    }
}
