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
        .type_attribute(
            ".logs_submission_schema.StoredEvent",
            "#[derive(::serde::Serialize)]",
        )
        .compile(
            &["event.proto", "logs/submission.proto"],
            &["./schema", "../../lib/ipc/proto"],
        )
        .context("compiling proto definitions")?;

    Ok(())
}
