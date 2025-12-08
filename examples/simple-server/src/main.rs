//! Simple xDS Server Example
//!
//! This example demonstrates a basic xDS control plane that:
//! - Creates a cache with some sample resources
//! - Sets up snapshots for nodes
//! - Runs a gRPC server (placeholder until xds-types has full protos)
//!
//! Run with:
//! ```bash
//! cargo run --example simple-server
//! ```

use std::sync::Arc;
use std::time::Duration;

use nebucloud_xds::prelude::*;
use tokio::signal;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

/// Configuration for the example server.
struct Config {
    /// Address to listen on.
    listen_addr: String,
    /// Number of sample clusters to create.
    num_clusters: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen_addr: "[::]:18000".to_string(),
            num_clusters: 3,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting simple xDS server example");
    info!("{}", nebucloud_xds::version::version_string());

    let config = Config::default();

    // Create the cache
    let cache = Arc::new(ShardedCache::new());
    info!("Created snapshot cache");

    // Create sample snapshots for different node types
    setup_sample_data(&cache, &config);

    // Create the xDS server
    let server = XdsServer::builder()
        .cache(Arc::clone(&cache))
        .enable_sotw()
        .enable_delta()
        .build()?;

    info!("xDS server configured");
    info!("  SotW enabled: {}", server.config().enable_sotw);
    info!("  Delta enabled: {}", server.config().enable_delta);

    // Start a background task to periodically update snapshots
    let cache_clone = Arc::clone(&cache);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        let mut version = 1u64;

        loop {
            interval.tick().await;
            version += 1;

            // Update snapshots
            let snapshot = Snapshot::builder()
                .version(format!("v{}", version))
                .resources(TypeUrl::new(TypeUrl::CLUSTER), vec![])
                .build();

            cache_clone.set_snapshot(NodeHash::from_id("edge-proxy"), snapshot);
            info!("Updated snapshots to version v{}", version);
        }
    });

    // Print cache stats periodically
    let cache_clone = Arc::clone(&cache);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));

        loop {
            interval.tick().await;
            let stats = cache_clone.stats();
            info!(
                "Cache stats: hits={}, misses={}, hit_rate={:.2}%",
                stats.snapshot_hits(),
                stats.snapshot_misses(),
                stats.hit_rate() * 100.0
            );
        }
    });

    info!(
        "Would listen on {} (server startup placeholder)",
        config.listen_addr
    );
    info!("Press Ctrl+C to shutdown");

    // Wait for shutdown signal
    signal::ctrl_c().await?;

    info!("Shutting down...");
    Ok(())
}

/// Set up sample data in the cache.
fn setup_sample_data(cache: &ShardedCache, config: &Config) {
    // Create snapshots for different node types

    // Edge proxy nodes
    let edge_snapshot = Snapshot::builder()
        .version("v1")
        .resources(TypeUrl::new(TypeUrl::CLUSTER), vec![])
        .resources(TypeUrl::new(TypeUrl::LISTENER), vec![])
        .build();

    cache.set_snapshot(NodeHash::from_id("edge-proxy"), edge_snapshot);
    info!("Set snapshot for edge-proxy nodes");

    // Internal service mesh nodes
    let mesh_snapshot = Snapshot::builder()
        .version("v1")
        .resources(TypeUrl::new(TypeUrl::CLUSTER), vec![])
        .resources(TypeUrl::new(TypeUrl::ENDPOINT), vec![])
        .build();

    cache.set_snapshot(NodeHash::from_id("mesh-sidecar"), mesh_snapshot);
    info!("Set snapshot for mesh-sidecar nodes");

    // Gateway nodes
    let gateway_snapshot = Snapshot::builder()
        .version("v1")
        .resources(TypeUrl::new(TypeUrl::LISTENER), vec![])
        .resources(TypeUrl::new(TypeUrl::ROUTE), vec![])
        .resources(TypeUrl::new(TypeUrl::CLUSTER), vec![])
        .build();

    cache.set_snapshot(NodeHash::from_id("gateway"), gateway_snapshot);
    info!("Set snapshot for gateway nodes");

    info!(
        "Initialized cache with {} node types, {} clusters configured",
        cache.snapshot_count(),
        config.num_clusters
    );
}
