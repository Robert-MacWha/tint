pub fn ccs_bytes() -> std::io::Result<Vec<u8>> {
    read_artifact("ccs.bin")
}

pub fn proving_key_bytes() -> std::io::Result<Vec<u8>> {
    read_artifact("proving_key.bin")
}

pub fn verifying_key_bytes() -> std::io::Result<Vec<u8>> {
    read_artifact("verifying_key.bin")
}

fn read_artifact(name: &str) -> std::io::Result<Vec<u8>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("artifacts")
        .join(name);
    std::fs::read(&path)
}
