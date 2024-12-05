fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile(
            &["proto/rustray.proto"],
            &["proto"],
        )
        .unwrap_or_else(|e| panic!("Failed to compile protos {:?}", e));
    Ok(())
} 