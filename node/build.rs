use std::io::Result;
fn main() -> Result<()> {
    prost_build::compile_protos(
        &[
            "src/proto/peerland.proto",
            "src/proto/message/addressed.proto",
            "src/proto/message/broadcast.proto",
            "src/proto/time/logical.proto",
        ],
        &["src/proto"],
    )?;
    Ok(())
}
