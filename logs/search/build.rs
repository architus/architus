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
    // Compile the logs/event protobuf definitions,
    // adding serde, serde_repr, strum, and juniper derives as needed.
    let mut builder = tonic_build::configure()
        .build_client(false)
        .build_server(false);

    for &message_path in MESSAGES {
        builder = builder.type_attribute(message_path, "#[derive(::serde::Deserialize)]");
    }

    for &enum_path in ENUMS {
        builder = builder.type_attribute(enum_path, "#[derive(::serde_repr::Deserialize_repr)]");

        // Note: we have to hope (and pray) that the strum serializations are the same as the juniper ones
        // The book (https://graphql-rust.github.io/juniper/current/types/enums.html#enums) is wrong
        // when it says they are uppercase; they should be SCREAMING_SNAKE_CASE in juniper as well
        builder = builder
            .type_attribute(enum_path, "#[derive(::juniper::GraphQLEnum)]")
            .type_attribute(enum_path, "#[derive(::strum::EnumString)]")
            .type_attribute(
                enum_path,
                r#"#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]"#,
            );
    }

    builder
        .compile(
            &["event.proto", "logs/event.proto"],
            &["../submission/schema", "../../lib/ipc/proto"],
        )
        .context("compiling proto definitions")?;

    Ok(())
}
