use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Command::new("cp")
        .arg("../lib/ipc/proto/feature-gate.proto")
        .arg("./proto")
        .status()?;

    tonic_build::configure()
        .build_client(false)
        .build_server(true)
        .format(true)
        .out_dir("./grpc")
        .compile(&["feature-gate.proto"], &["./proto"])?;

    Ok(())
}
