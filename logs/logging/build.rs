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
        .compile(&["logging.proto"], &["../../lib/ipc/proto"])
        .context("Compiling logging.proto definitions")?;

    Ok(())
}
