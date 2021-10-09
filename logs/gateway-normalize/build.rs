use anyhow::Context;

fn main() -> anyhow::Result<()> {
    // Compile the gateway-queue-lib protobuf definitions
    tonic_build::configure()
        .build_client(false)
        .build_server(false)
        .compile(&["event.proto"], &["../gateway-queue-lib/proto"])
        .context("compiling logs/gateway-queue-lib/proto/event.proto definitions")?;

    Ok(())
}
