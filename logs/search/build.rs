use anyhow::{Context, Result};

fn main() -> Result<()> {
    // Compile the logging protobuf definitions into the server code
    tonic_build::configure()
        .build_client(false)
        .build_server(true)
        .type_attribute(
            ".Logging.EventOrigin",
            "#[derive(::serde_repr::Serialize_repr, ::serde_repr::Deserialize_repr)]",
        )
        .type_attribute(
            ".Logging.EventType",
            "#[derive(::serde_repr::Serialize_repr, ::serde_repr::Deserialize_repr)]",
        )
        .type_attribute(".Logging.EventOrigin", "#[derive(::juniper::GraphQLEnum)]")
        .type_attribute(".Logging.EventType", "#[derive(::juniper::GraphQLEnum)]")
        .type_attribute(".Logging.EventOrigin", "#[derive(::strum::EnumString)]")
        .type_attribute(".Logging.EventType", "#[derive(::strum::EnumString)]")
        // Note: we have to hope (and pray) that the strum serializations are the same as the juniper ones
        // The book (https://graphql-rust.github.io/juniper/current/types/enums.html#enums) is wrong
        // when it says they are uppercase; they should be SCREAMING_SNAKE_CASE in juniper as well
        .type_attribute(
            ".Logging.EventOrigin",
            r#"#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]"#,
        )
        .type_attribute(
            ".Logging.EventType",
            r#"#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]"#,
        )
        .compile(&["logging.proto"], &["../../lib/ipc/proto"])
        .context("Compiling logging.proto definitions")?;

    Ok(())
}
