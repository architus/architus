use anyhow::{Context, Result};

fn main() -> Result<()> {
    // Compile the logging protobuf definitions into the server code
    tonic_build::configure()
        .build_client(false)
        .build_server(true)
        .type_attribute(".", "#[derive(serde::Serialize)]")
        .compile(&["logging.proto"], &["../../lib/ipc/proto"])
        .context("Compiling logging.proto definitions")?;

    Ok(())
}
