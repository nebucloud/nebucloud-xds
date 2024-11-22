use std::sync::Arc;
use std::time::Instant;

use tokio::sync::mpsc;
use tokio::time::Duration;
use tracing::info;

use crate::model::{to_snapshot, Cluster, Endpoint};
use crate::process::EnvoyProcess;
use rust_control_plane::cache::snapshot::SnapshotCache;

pub async fn stress_test() { 
    // Initialize the snapshot cache
    let cache = Arc::new(SnapshotCache::new(false));

    // Create a channel for communication with the Envoy process
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

    // Spawn a task to simulate Envoy and receive updates
    tokio::spawn(async move {
        let mut envoy = EnvoyProcess::new(false, false);
        envoy.spawn().unwrap();
        envoy.poll_until_started().await.unwrap();

        // Wait for the shutdown signal
        let _ = shutdown_rx;
    });

    // Generate a large number of clusters and endpoints
    let num_clusters = 1000;
    let num_endpoints_per_cluster = 100;
    let clusters = generate_clusters(num_clusters, num_endpoints_per_cluster);

    // Record the start time
    let start_time = Instant::now();

    // Set the initial snapshot
    cache
        .set_snapshot("lol", to_snapshot(&clusters, "init", false))
        .await;

    // Continuously update the snapshot with modified clusters
    for i in 0..100 {
        let updated_clusters = modify_clusters(clusters.clone(), i);
        cache
            .set_snapshot(
                "lol",
                to_snapshot(&updated_clusters, &format!("update-{}", i), false),
            )
            .await;

        // Sleep for a short duration to simulate real-world update frequency
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Record the end time
    let end_time = Instant::now();

    // Send the shutdown signal to the Envoy task
    shutdown_tx.send(()).await.unwrap();

    // Calculate and log the total time taken
    let total_time = end_time.duration_since(start_time);
    info!(
        "Stress test completed in {:?} for {} clusters with {} endpoints each",
        total_time, num_clusters, num_endpoints_per_cluster
    );
}

fn generate_clusters(num_clusters: usize, num_endpoints_per_cluster: usize) -> Vec<Cluster> {
    (0..num_clusters)
        .map(|i| Cluster {
            name: format!("cluster-{}", i),
            hidden: false,
            endpoints: (0..num_endpoints_per_cluster)
                .map(|j| Endpoint {
                    addr: format!("10.0.0.{}", j),
                    port: 8080,
                })
                .collect(),
        })
        .collect()
}

fn modify_clusters(mut clusters: Vec<Cluster>, iteration: usize) -> Vec<Cluster> {
    for cluster in &mut clusters {
        for endpoint in &mut cluster.endpoints {
            endpoint.port = 8080 + iteration as u32;
        }
    }
    clusters
}
