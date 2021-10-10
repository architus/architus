use anyhow::Context;

const MESSAGES: &[&str] = &[
    ".logs.event.Event",
    ".logs.event.EventSource",
    ".logs.event.ContentMetadata",
    ".logs_submission_schema.StoredEvent",
];

const ENUMS: &[&str] = &[
    ".logs.event.EventOrigin",
    ".logs.event.EventType",
    ".logs.event.EntityType",
    ".logs.event.AgentSpecialType",
];

fn main() -> anyhow::Result<()> {
    // Compile the logs/submission protobuf definitions
    // (including the gRPC server code),
    // adding serde and serde_repr derives as needed.
    let mut builder = tonic_build::configure()
        .build_client(false)
        .build_server(true);

    for &message_path in MESSAGES {
        builder = builder.type_attribute(message_path, "#[derive(::serde::Serialize)]");
    }

    for &enum_path in ENUMS {
        builder = builder.type_attribute(enum_path, "#[derive(::serde_repr::Serialize_repr)]");
    }

    builder
        .compile(
            &["event.proto", "logs/submission.proto"],
            &["./schema", "../../lib/ipc/proto"],
        )
        .context("compiling proto definitions")?;

    Ok(())
}
