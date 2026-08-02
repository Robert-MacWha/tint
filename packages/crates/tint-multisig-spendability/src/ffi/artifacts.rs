/// Reads artifacts/ccs.bin, written by `go run ./cmd/setup`.
pub fn ccs_bytes() -> Vec<u8> {
    read_artifact("ccs.bin")
}

/// Reads artifacts/proving_key.bin, written by `go run ./cmd/setup`.
pub fn proving_key_bytes() -> Vec<u8> {
    read_artifact("proving_key.bin")
}

/// Reads artifacts/verifying_key.bin, written by `go run ./cmd/setup`.
pub fn verifying_key_bytes() -> Vec<u8> {
    read_artifact("verifying_key.bin")
}

fn read_artifact(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("artifacts")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "{}: {e} (run `go run ./cmd/setup` from go/ first)",
            path.display()
        )
    })
}
