use anyhow::Context;

fn main() -> anyhow::Result<()> {
    // Compile the logs/submission protobuf definitions into the client code
    tonic_build::configure()
        .build_client(true)
        .build_server(false)
        .compile(&["logs/submission.proto"], &["../../lib/ipc/proto"])
        .context("compiling logs/submission.proto definitions")?;
    // Compile the gateway-queue-lib protobuf definitions
    tonic_build::configure()
        .build_client(false)
        .build_server(false)
        .compile(&["event.proto"], &["../gateway-queue-lib/proto"])
        .context("compiling logs/gateway-queue-lib/proto/event.proto definitions")?;

    Ok(())
}
