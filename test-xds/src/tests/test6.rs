use crate::model::{to_snapshot, Cluster, Endpoint};
use crate::process::EnvoyProcess;
use rust_control_plane::cache::snapshot::SnapshotCache;
use std::sync::Arc;
use tracing::info;

// 1. Begin with a snapshot of one cluster.
// 2. Update the cluster to be hidden.

pub fn init() -> Option<Vec<Cluster>> {
    Some(vec![Cluster {
        name: "cluster-0".to_string(),
        hidden: false,
        endpoints: vec![Endpoint {
            addr: "127.0.0.1".to_string(),
            port: 1234,
        }],
    }])
}

pub async fn test(cache: Arc<SnapshotCache>, envoy: EnvoyProcess, ads: bool) {
    envoy.poll_until_eq(init().unwrap()).await.unwrap();
    info!("init equal");

    let snapshot1 = vec![Cluster {
        name: "cluster-0".to_string(),
        hidden: true, // Now hidden
        endpoints: vec![Endpoint {
            addr: "127.0.0.1".to_string(),
            port: 1234,
        }],
    }];

    info!("setting snapshot");
    cache
        .set_snapshot("lol", to_snapshot(&snapshot1, "snapshot1", ads))
        .await;

    // Expect an empty set of clusters since the only cluster is now hidden
    let expected_clusters = vec![];
    envoy.poll_until_eq(expected_clusters).await.unwrap();
    info!("snapshot equal");
}
