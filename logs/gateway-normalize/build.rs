use anyhow::{Context, Result};

fn main() -> Result<()> {
    // Compile the logs/submission protobuf definitions into the client code
    tonic_build::configure()
        .build_client(true)
        .build_server(false)
        .compile(&["logs/submission.proto"], &["../../lib/ipc/proto"])
        .context("compiling logs/submission.proto definitions")?;

    Ok(())
}
