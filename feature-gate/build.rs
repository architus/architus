fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile the feature-gate protobuf definitions into the server code
    tonic_build::configure()
        .build_client(false)
        .build_server(true)
        .compile(&["feature-gate.proto"], &["../lib/proto"])?;

    Ok(())
}
