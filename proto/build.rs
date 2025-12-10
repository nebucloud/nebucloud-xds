use glob::glob;
use std::io;
use std::path::PathBuf;

fn main() -> io::Result<()> {
    // The data-plane-api submodule contains the Envoy proto definitions
    let protos: Vec<PathBuf> = glob("data-plane-api/envoy/**/v3/*.proto")
        .expect("Failed to read proto glob pattern")
        .filter_map(Result::ok)
        .collect();
    let mut config = prost_build::Config::new();
    config.disable_comments(["."]);
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_well_known_types(true)
        .include_file("mod.rs")
        .compile_protos_with_config(
            config,
            &protos,
            &[
                "data-plane-api",  // Envoy data-plane-api submodule
                "googleapis",
                "protoc-gen-validate",
                "xds",
                "opencensus-proto/src",
                "opentelemetry-proto",
                "client_model",
                "cel-spec/proto",
            ],
        )?;
    Ok(())
}
