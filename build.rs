fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("lib/eigenda/api/proto/disperser/disperser.proto")?;
    Ok(())
}
