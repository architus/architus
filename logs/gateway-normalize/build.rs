use anyhow::{Context, Result};

fn main() -> Result<()> {
    // Compile the logs/submission protobuf definitions into the client code
    tonic_build::configure()
        .build_client(true)
        .build_server(false)
        .compile(&["submission.proto"], &["../../lib/ipc/proto/logs"])
        .context("Compiling submission.proto definitions")?;

    Ok(())
}
