use std::io::Result;
fn main() -> Result<()> {
    prost_build::compile_protos(
        &[
            "src/communication/proto/peerland.proto",
            "src/communication/proto/message/addressed.proto",
            "src/communication/proto/message/broadcast.proto",
            "src/communication/proto/time/logical.proto",
        ],
        &["src/communication/proto"],
    )?;
    Ok(())
}
