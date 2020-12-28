use anyhow::{Context, Result};

fn main() -> Result<()> {
    // Compile the logs/submission protobuf definitions into the server code
    tonic_build::configure()
        .build_client(false)
        .build_server(true)
        .type_attribute(
            ".logs.submission.EventOrigin",
            "#[derive(::serde_repr::Serialize_repr, ::serde_repr::Deserialize_repr)]",
        )
        .type_attribute(
            ".logs.submission.EventType",
            "#[derive(::serde_repr::Serialize_repr, ::serde_repr::Deserialize_repr)]",
        )
        .compile(&["submission.proto"], &["../../lib/ipc/proto/logs"])
        .context("Compiling submission.proto definitions")?;
    // Compile the logs/revision protobuf definitions into the client code
    tonic_build::configure()
        .build_client(true)
        .build_server(false)
        .compile(&["revision.proto"], &["../../lib/ipc/proto/logs"])
        .context("Compiling revision.proto definitions")?;

    Ok(())
}
