#[derive(Clone, Debug)]
pub struct Artifacts {
    pub ccs: Vec<u8>,
    pub pk: Vec<u8>,
    pub vk: Vec<u8>,
}

pub fn load_artifacts() -> std::io::Result<Artifacts> {
    Ok(Artifacts {
        ccs: load_ccs()?,
        pk: load_pk()?,
        vk: load_vk()?,
    })
}

pub fn load_ccs() -> std::io::Result<Vec<u8>> {
    read_artifact("ccs.bin")
}

pub fn load_pk() -> std::io::Result<Vec<u8>> {
    read_artifact("proving_key.bin")
}

pub fn load_vk() -> std::io::Result<Vec<u8>> {
    read_artifact("verifying_key.bin")
}

fn read_artifact(name: &str) -> std::io::Result<Vec<u8>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("artifacts")
        .join(name);
    std::fs::read(&path)
}
