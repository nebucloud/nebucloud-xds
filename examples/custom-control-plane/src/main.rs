//! Custom Control Plane Example
//!
//! This example demonstrates how to build a domain-specific xDS control plane
//! using nebucloud-xds as a library. It shows the pattern that external consumers
//! (like Dove Express for trucking dispatch) would follow.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                        Your Application                                  │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌──────────────┐    ┌──────────────┐    ┌──────────────────────────┐  │
//! │  │   Domain     │───▶│  Conversion  │───▶│  nebucloud-xds           │  │
//! │  │   Types      │    │  Layer       │    │  (Cache + Server)        │  │
//! │  │              │    │              │    │                          │  │
//! │  │  - Load      │    │  - ToCluster │    │  - ShardedCache          │  │
//! │  │  - Vehicle   │    │  - ToEndpoint│    │  - XdsServer             │  │
//! │  │  - Assignment│    │  - ToRoute   │    │  - Snapshot              │  │
//! │  └──────────────┘    └──────────────┘    └──────────────────────────┘  │
//! │         │                                           │                   │
//! │         │            ┌──────────────┐               │                   │
//! │         └───────────▶│   Snapshot   │◀──────────────┘                   │
//! │                      │   Builder    │                                   │
//! │                      └──────────────┘                                   │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//!                                    │
//!                                    ▼ xDS Protocol (gRPC)
//!                           ┌────────────────┐
//!                           │  Envoy Proxies │
//!                           │  (Vehicles,    │
//!                           │   Depots, etc) │
//!                           └────────────────┘
//! ```
//!
//! ## Key Concepts
//!
//! 1. **Domain Types** (`domain.rs`): Your business objects (Load, Vehicle, Assignment)
//! 2. **Conversion** (`conversion.rs`): Transform domain → xDS resources
//! 3. **Snapshot** (`snapshot.rs`): Build consistent xDS snapshots from domain state
//! 4. **Server**: Use nebucloud-xds to serve snapshots to Envoy
//!
//! ## Running
//!
//! ```bash
//! cargo run -p custom-control-plane-example
//! ```

mod conversion;
mod domain;
mod snapshot;

use std::sync::{Arc, RwLock};
use std::time::Duration;

use domain::{Assignment, Load, Vehicle};
use nebucloud_xds::prelude::*;
use snapshot::DispatchState;
use tokio::signal;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

/// Configuration for the dispatch control plane.
struct Config {
    /// gRPC listen address.
    listen_addr: String,
    /// How often to refresh snapshots.
    refresh_interval: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen_addr: "[::]:18000".to_string(),
            refresh_interval: Duration::from_secs(5),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔═══════════════════════════════════════════════════════════╗");
    info!("║     Custom Control Plane Example - Logistics Dispatch     ║");
    info!("║                                                           ║");
    info!("║  This demonstrates how to build a domain-specific xDS     ║");
    info!("║  control plane using nebucloud-xds as a library.          ║");
    info!("╚═══════════════════════════════════════════════════════════╝");
    info!("");
    info!("{}", nebucloud_xds::version::version_string());

    let config = Config::default();

    // ========================================================================
    // Step 1: Create the xDS cache
    // ========================================================================
    let cache = Arc::new(ShardedCache::new());
    info!("✓ Created xDS snapshot cache");

    // ========================================================================
    // Step 2: Initialize domain state
    // ========================================================================
    let dispatch_state = Arc::new(RwLock::new(DispatchState::new()));
    info!("✓ Initialized dispatch state manager");

    // Load sample data
    load_sample_data(&dispatch_state);

    // ========================================================================
    // Step 3: Build and set initial snapshots
    // ========================================================================
    refresh_snapshots(&dispatch_state, &cache);
    info!("✓ Built initial xDS snapshots");

    // ========================================================================
    // Step 4: Create the xDS server
    // ========================================================================
    let server = XdsServer::builder()
        .cache(Arc::clone(&cache))
        .enable_sotw()  // State-of-the-World protocol
        .enable_delta() // Delta (incremental) protocol
        .build()?;

    info!("✓ xDS server configured");
    info!("  └─ SotW enabled: {}", server.config().enable_sotw);
    info!("  └─ Delta enabled: {}", server.config().enable_delta);

    // ========================================================================
    // Step 5: Start background tasks
    // ========================================================================

    // Task: Periodically refresh snapshots (simulates domain state changes)
    let dispatch_state_clone = Arc::clone(&dispatch_state);
    let cache_clone = Arc::clone(&cache);
    let refresh_interval = config.refresh_interval;

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(refresh_interval);
        let mut counter = 0u64;

        loop {
            interval.tick().await;
            counter += 1;

            // Simulate domain updates (in real app, this comes from your API)
            if counter % 3 == 0 {
                info!("🔄 Simulating domain update...");
            }

            // Rebuild and push snapshots
            refresh_snapshots(&dispatch_state_clone, &cache_clone);
        }
    });

    // Task: Log cache statistics
    let cache_stats = Arc::clone(&cache);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));

        loop {
            interval.tick().await;
            let stats = cache_stats.stats();
            info!(
                "📊 Cache stats: hits={}, misses={}, hit_rate={:.1}%",
                stats.snapshot_hits(),
                stats.snapshot_misses(),
                stats.hit_rate() * 100.0
            );
        }
    });

    // ========================================================================
    // Step 6: Run the server (placeholder - actual gRPC would go here)
    // ========================================================================
    info!("");
    info!("🚀 Control plane ready!");
    info!("   Would listen on: {}", config.listen_addr);
    info!("");
    info!("   Node types:");
    info!("   ├─ dispatcher-node  → Full dispatch view (loads, vehicles, routes)");
    info!("   ├─ vehicle-*        → Per-vehicle assignments");
    info!("   └─ depot-*          → Regional vehicle fleet");
    info!("");
    info!("Press Ctrl+C to shutdown");

    // Wait for shutdown
    signal::ctrl_c().await?;
    info!("Shutting down gracefully...");

    Ok(())
}

/// Load sample data into the dispatch state.
fn load_sample_data(state: &Arc<RwLock<DispatchState>>) {
    let mut state = state.write().unwrap();

    info!("Loading sample data...");

    // Add sample loads
    state.upsert_load(
        Load::new("load-001", "Refrigerated produce - Dallas to Houston")
            .origin(32.7767, -96.7970, "Dallas, TX")
            .destination(29.7604, -95.3698, "Houston, TX")
            .weight(18000)
            .require("refrigerated")
            .priority(8),
    );

    state.upsert_load(
        Load::new("load-002", "Construction materials - Phoenix to LA")
            .origin(33.4484, -112.0740, "Phoenix, AZ")
            .destination(34.0522, -118.2437, "Los Angeles, CA")
            .weight(35000)
            .require("flatbed")
            .priority(5),
    );

    state.upsert_load(
        Load::new("load-003", "General freight - Atlanta to Miami")
            .origin(33.7490, -84.3880, "Atlanta, GA")
            .destination(25.7617, -80.1918, "Miami, FL")
            .weight(25000)
            .priority(6),
    );

    info!("  ✓ Added 3 sample loads");

    // Add sample vehicles
    state.upsert_vehicle(
        Vehicle::new("truck-001", "John Doe")
            .at(32.7767, -96.7970)
            .capacity(45000)
            .with_capability("refrigerated")
            .health(0.95),
    );

    state.upsert_vehicle(
        Vehicle::new("truck-002", "Jane Smith")
            .at(33.4484, -112.0740)
            .capacity(50000)
            .with_capability("flatbed")
            .health(0.88),
    );

    state.upsert_vehicle(
        Vehicle::new("truck-003", "Bob Johnson")
            .at(33.7490, -84.3880)
            .capacity(45000)
            .health(0.92),
    );

    state.upsert_vehicle(
        Vehicle::new("truck-004", "Alice Williams")
            .at(29.7604, -95.3698)
            .capacity(45000)
            .with_capability("refrigerated")
            .health(0.97),
    );

    info!("  ✓ Added 4 sample vehicles");

    // Add initial assignments
    state.add_assignment(Assignment {
        id: "assign-001".to_string(),
        load_id: "load-001".to_string(),
        vehicle_id: "truck-001".to_string(),
        estimated_arrival: "2025-12-09T14:30:00Z".to_string(),
        route_points: vec![],
        weight: 100,
    });

    info!("  ✓ Added 1 sample assignment");
}

/// Refresh all snapshots from current domain state.
fn refresh_snapshots(state: &Arc<RwLock<DispatchState>>, cache: &ShardedCache) {
    let state = state.read().unwrap();

    // Snapshot for dispatcher nodes (full view)
    let dispatcher_snapshot = state.build_dispatcher_snapshot();
    cache.set_snapshot(NodeHash::from_id("dispatcher-node"), dispatcher_snapshot);

    // Snapshots for individual vehicles
    for vehicle_id in ["truck-001", "truck-002", "truck-003", "truck-004"] {
        let vehicle_snapshot = state.build_vehicle_snapshot(vehicle_id);
        cache.set_snapshot(NodeHash::from_id(vehicle_id), vehicle_snapshot);
    }

    // Snapshots for regional depots
    for region in ["us-west", "us-central", "us-east"] {
        let depot_snapshot = state.build_depot_snapshot(region);
        cache.set_snapshot(
            NodeHash::from_id(&format!("depot-{}", region)),
            depot_snapshot,
        );
    }
}
