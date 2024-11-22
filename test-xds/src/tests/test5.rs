use crate::model::{to_snapshot, Cluster, Endpoint};
use crate::process::EnvoyProcess;
use rust_control_plane::cache::snapshot::SnapshotCache;
use std::sync::Arc;
use tracing::info;

// 1. Begin with a snapshot of one cluster with two endpoints.
// 2. Update the cluster to have only one of the original endpoints.

pub fn init() -> Option<Vec<Cluster>> {
    Some(vec![Cluster {
        name: "cluster-0".to_string(),
        hidden: false,
        endpoints: vec![
            Endpoint {
                addr: "127.0.0.1".to_string(),
                port: 1234,
            },
            Endpoint {
                addr: "127.0.0.1".to_string(),
                port: 5678,
            },
        ],
    }])
}

pub async fn test(cache: Arc<SnapshotCache>, envoy: EnvoyProcess, ads: bool) {
    envoy.poll_until_eq(init().unwrap()).await.unwrap();
    info!("init equal");

    let snapshot1 = vec![Cluster {
        name: "cluster-0".to_string(),
        hidden: false,
        endpoints: vec![Endpoint {
            // Removed one endpoint
            addr: "127.0.0.1".to_string(),
            port: 1234,
        }],
    }];

    info!("setting snapshot");
    cache
        .set_snapshot("lol", to_snapshot(&snapshot1, "snapshot1", ads))
        .await;
    envoy.poll_until_eq(snapshot1).await.unwrap();
    info!("snapshot equal");
}
