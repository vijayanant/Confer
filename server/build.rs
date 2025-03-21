fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .type_attribute(".", "#[derive(Eq, serde::Serialize, serde::Deserialize)]")
        .compile_protos(
            &["../proto/confer.proto", "src/raft/proto/raft.proto"],
            &["../proto", "src/raft/proto"],
        )?;
    Ok(())
}
