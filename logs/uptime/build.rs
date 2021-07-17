use anyhow::Context;

fn main() -> anyhow::Result<()> {
    // Compile the logs/uptime protobuf definitions into the server code
    tonic_build::configure()
        .build_client(false)
        .build_server(true)
        .compile(&["uptime.proto"], &["../../lib/ipc/proto/logs"])
        .context("compiling uptime.proto definitions")?;

    Ok(())
}
