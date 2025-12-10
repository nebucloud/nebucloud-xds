//! Benchmarks for xds-cache operations.
//!
//! Run with: `cargo bench --package xds-cache`
//!
//! These benchmarks measure:
//! - Snapshot set/get operations
//! - Watch creation and notification
//! - Concurrent access patterns
//! - Scaling with number of nodes

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use xds_cache::{Cache, ShardedCache, Snapshot};
use xds_core::{NodeHash, TypeUrl};

/// Create a sample snapshot with the given number of resource types.
fn create_snapshot(version: &str, num_types: usize) -> Snapshot {
    let mut builder = Snapshot::builder().version(version);

    // Add empty resource collections for different types
    let types = [
        TypeUrl::CLUSTER,
        TypeUrl::LISTENER,
        TypeUrl::ROUTE,
        TypeUrl::ENDPOINT,
        TypeUrl::SECRET,
    ];

    for type_url in types.iter().take(num_types) {
        builder = builder.resources(TypeUrl::new(*type_url), vec![]);
    }

    builder.build()
}

/// Benchmark snapshot set operations.
fn bench_set_snapshot(c: &mut Criterion) {
    let mut group = c.benchmark_group("set_snapshot");

    for num_nodes in [1, 10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*num_nodes as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(num_nodes),
            num_nodes,
            |b, &num_nodes| {
                let cache = ShardedCache::new();
                let nodes: Vec<NodeHash> = (0..num_nodes)
                    .map(|i| NodeHash::from_id(&format!("node-{}", i)))
                    .collect();
                let snapshot = create_snapshot("v1", 3);

                b.iter(|| {
                    for node in &nodes {
                        cache.set_snapshot(*node, snapshot.clone());
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark snapshot get operations (cache hits).
fn bench_get_snapshot_hit(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_snapshot_hit");

    for num_nodes in [1, 10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*num_nodes as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(num_nodes),
            num_nodes,
            |b, &num_nodes| {
                let cache = ShardedCache::new();
                let nodes: Vec<NodeHash> = (0..num_nodes)
                    .map(|i| NodeHash::from_id(&format!("node-{}", i)))
                    .collect();

                // Pre-populate cache
                for node in &nodes {
                    cache.set_snapshot(*node, create_snapshot("v1", 3));
                }

                b.iter(|| {
                    for node in &nodes {
                        black_box(cache.get_snapshot(*node));
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark snapshot get operations (cache misses).
fn bench_get_snapshot_miss(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_snapshot_miss");

    for num_nodes in [1, 10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*num_nodes as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(num_nodes),
            num_nodes,
            |b, &num_nodes| {
                let cache = ShardedCache::new();
                let nodes: Vec<NodeHash> = (0..num_nodes)
                    .map(|i| NodeHash::from_id(&format!("node-{}", i)))
                    .collect();

                // Don't populate cache - measure miss performance
                b.iter(|| {
                    for node in &nodes {
                        black_box(cache.get_snapshot(*node));
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark watch creation.
fn bench_create_watch(c: &mut Criterion) {
    let mut group = c.benchmark_group("create_watch");

    for num_watches in [1, 10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*num_watches as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(num_watches),
            num_watches,
            |b, &num_watches| {
                let cache = ShardedCache::new();
                let nodes: Vec<NodeHash> = (0..num_watches)
                    .map(|i| NodeHash::from_id(&format!("node-{}", i)))
                    .collect();

                b.iter(|| {
                    for node in &nodes {
                        let watch = cache.create_watch(*node);
                        black_box(watch);
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark snapshot builder with different resource counts.
fn bench_snapshot_builder(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshot_builder");

    for num_types in [1, 3, 5].iter() {
        group.bench_with_input(
            BenchmarkId::new("types", num_types),
            num_types,
            |b, &num_types| {
                b.iter(|| {
                    black_box(create_snapshot("v1", num_types));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark mixed read/write workload.
fn bench_mixed_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_workload");

    // 90% reads, 10% writes
    group.bench_function("90_read_10_write", |b| {
        let cache = ShardedCache::new();
        let num_nodes = 100;
        let nodes: Vec<NodeHash> = (0..num_nodes)
            .map(|i| NodeHash::from_id(&format!("node-{}", i)))
            .collect();

        // Pre-populate
        for node in &nodes {
            cache.set_snapshot(*node, create_snapshot("v1", 3));
        }

        let mut counter = 0u64;
        b.iter(|| {
            counter += 1;
            let node = &nodes[(counter as usize) % num_nodes];

            if counter % 10 == 0 {
                // 10% writes
                cache.set_snapshot(*node, create_snapshot(&format!("v{}", counter), 3));
            } else {
                // 90% reads
                black_box(cache.get_snapshot(*node));
            }
        });
    });

    group.finish();
}

/// Benchmark NodeHash creation.
fn bench_node_hash(c: &mut Criterion) {
    let mut group = c.benchmark_group("node_hash");

    group.bench_function("from_id_short", |b| {
        b.iter(|| {
            black_box(NodeHash::from_id("node-1"));
        });
    });

    group.bench_function("from_id_long", |b| {
        let long_id = "envoy-sidecar-pod-name-with-namespace.namespace.svc.cluster.local";
        b.iter(|| {
            black_box(NodeHash::from_id(long_id));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_set_snapshot,
    bench_get_snapshot_hit,
    bench_get_snapshot_miss,
    bench_create_watch,
    bench_snapshot_builder,
    bench_mixed_workload,
    bench_node_hash,
);

criterion_main!(benches);
