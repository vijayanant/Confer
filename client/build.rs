fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false)
        //.out_dir("./src")
        .compile_protos(&["proto/confer.proto"], &["proto"])?;
    Ok(())
}
