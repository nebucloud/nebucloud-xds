//! Build script for xds-types.
//!
//! This script compiles Envoy xDS protobuf definitions into Rust types
//! using tonic-build and prost-build.

use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);

    // Check if we have proto files to compile
    // For now, we'll use pre-generated types or skip if protos aren't available
    let proto_dir = PathBuf::from("proto");

    if proto_dir.exists() {
        compile_protos(&proto_dir, &out_dir)?;
    } else {
        // Generate stub types when protos aren't available
        generate_stub_types(&out_dir)?;
    }

    Ok(())
}

#[allow(dead_code)]
fn compile_protos(proto_dir: &PathBuf, out_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // Proto files to compile
    let protos: Vec<PathBuf> = [
        "envoy/service/discovery/v3/ads.proto",
        "envoy/service/cluster/v3/cds.proto",
        "envoy/service/listener/v3/lds.proto",
        "envoy/service/route/v3/rds.proto",
        "envoy/service/endpoint/v3/eds.proto",
        "envoy/service/secret/v3/sds.proto",
        "envoy/config/cluster/v3/cluster.proto",
        "envoy/config/listener/v3/listener.proto",
        "envoy/config/route/v3/route.proto",
        "envoy/config/endpoint/v3/endpoint.proto",
        "envoy/extensions/transport_sockets/tls/v3/secret.proto",
    ]
    .iter()
    .map(|p| proto_dir.join(p))
    .collect();

    // Include paths - store PathBufs to ensure they live long enough
    let googleapis = proto_dir.join("googleapis");
    let validate = proto_dir.join("protoc-gen-validate");
    let xds = proto_dir.join("xds");

    let includes: Vec<PathBuf> = vec![
        proto_dir.clone(),
        googleapis,
        validate,
        xds,
    ];

    // Configure tonic build
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .out_dir(out_dir)
        .compile_protos(&protos, &includes)?;

    Ok(())
}

fn generate_stub_types(out_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // Generate a marker file indicating stubs are being used
    let stub_marker = out_dir.join("STUB_TYPES");
    std::fs::write(&stub_marker, "Using stub types - proto files not available")?;

    Ok(())
}
