use std::collections::HashSet;
use std::sync::Arc;

use crate::model::{to_snapshot, Cluster, Endpoint};
use crate::process::EnvoyProcess;
use rust_control_plane::cache::snapshot::SnapshotCache;

// 1. Begin with a snapshot of one cluster with one endpoint.
// 2. Update the snapshot to have the same cluster but with a different endpoint.

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

pub async fn test(cache: Arc<SnapshotCache>, mut envoy: EnvoyProcess, _ads: bool) {
    envoy.poll_until_eq(init().unwrap()).await.unwrap();

    // Envoy fetches the /clusters endpoint twice, and the second time it does
    // this, it does not include the xds cluster.
    //
    // This is likely because the gRPC stream used for the xDS protocol is
    // closed by the server after the first fetch, and Envoy does not
    // re-establish the stream until it needs to fetch the /clusters endpoint
    // again.
    //
    // This is not a problem for the xDS protocol, as Envoy will still receive
    // updates to the xds cluster when they are pushed by the server.
    //
    // However, it is a problem for this test, as it expects the xds cluster
    // to be present in the second fetch.
    //
    // To work around this, we wait for the second fetch to occur before
    // setting the snapshot.
    envoy
        .wait_for_fetch(HashSet::from(["cluster-0".to_string()]))
        .await;
    envoy.wait_for_fetch(HashSet::new()).await;

    let snapshot1 = vec![Cluster {
        name: "cluster-0".to_string(),
        hidden: false,
        endpoints: vec![Endpoint {
            addr: "127.0.0.1".to_string(),
            port: 5678,
        }],
    }];

    cache
        .set_snapshot("lol", to_snapshot(&snapshot1, "snapshot1", false))
        .await;
    envoy.poll_until_eq(snapshot1).await.unwrap();
}
