fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:warning=OUT_DIR={}", std::env::var("OUT_DIR").unwrap());
    tonic_build::compile_protos("../proto/confer.proto")?;
    Ok(())
}
