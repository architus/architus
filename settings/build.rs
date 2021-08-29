fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_client(false)
        .build_server(true)
        .compile(&["settings.proto"], &["../lib/ipc/proto"])?;

    Ok(())
}
