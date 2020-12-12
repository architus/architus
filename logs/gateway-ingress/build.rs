use anyhow::{Context, Result};

fn main() -> Result<()> {
    // Compile the logging protobuf definitions into the client code
    tonic_build::configure()
        .build_client(true)
        .build_server(false)
        .compile(&["feature-gate.proto"], &["../../lib/ipc/proto"])
        .context("Compiling feature-gate.proto definitions")?;

    Ok(())
}
