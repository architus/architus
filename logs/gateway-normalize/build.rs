use anyhow::{Context, Result};

fn main() -> Result<()> {
    // Compile the logs/import protobuf definitions into the client code
    tonic_build::configure()
        .build_client(true)
        .build_server(false)
        .compile(&["import.proto"], &["../../lib/ipc/proto/logs"])
        .context("Compiling import.proto definitions")?;

    Ok(())
}
