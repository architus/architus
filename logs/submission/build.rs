use anyhow::Context;

fn main() -> anyhow::Result<()> {
    // Compile the logs/submission protobuf definitions into the server code
    tonic_build::configure()
        .build_client(false)
        .build_server(true)
        .type_attribute(
            ".logs.event.EventOrigin",
            "#[derive(::serde_repr::Serialize_repr, ::serde_repr::Deserialize_repr)]",
        )
        .type_attribute(
            ".logs.event.EventType",
            "#[derive(::serde_repr::Serialize_repr, ::serde_repr::Deserialize_repr)]",
        )
        .type_attribute(".logs.event.Event", "#[derive(::serde::Serialize)]")
        .type_attribute(".logs.event.EventSource", "#[derive(::serde::Serialize)]")
        .type_attribute(
            ".logs.event.ContentMetadata",
            "#[derive(::serde::Serialize)]",
        )
        .compile(&["logs/submission.proto"], &["../../lib/ipc/proto"])
        .context("compiling logs/submission.proto definitions")?;

    Ok(())
}
