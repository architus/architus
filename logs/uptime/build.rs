use anyhow::{Context, Result};

fn main() -> Result<()> {
    // Compile the logs/uptime protobuf definitions into the server code
    tonic_build::configure()
        .build_client(false)
        .build_server(true)
        .compile(&["uptime.proto"], &["../../lib/ipc/proto/logs"])
        .context("Compiling uptime.proto definitions")?;

    Ok(())
}
