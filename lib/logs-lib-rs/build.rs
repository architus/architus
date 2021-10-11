use anyhow::Context;

fn main() -> anyhow::Result<()> {
    // Compile the logs/submission protobuf definitions
    tonic_build::configure()
        .build_client(true)
        .build_server(false)
        .compile(&["logs/submission.proto"], &["../../lib/proto"])
        .context("compiling logs/submission.proto definitions")?;

    Ok(())
}
