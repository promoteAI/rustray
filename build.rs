use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .build_server(true)
        .build_client(true)
        .compile(
            &["src/proto/rustray.proto"],
            &["src/proto"],
        )?;
    Ok(())
} 