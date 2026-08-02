use ark_bn254::Fr;
use ark_ff::PrimeField;
use k256::ecdsa::VerifyingKey;
use tint::circuit::poseidon2::poseidon2_compress;

#[derive(Clone, Copy, Debug, thiserror::Error)]
#[error("invalid secp256k1 public key encoding")]
pub struct InvalidPublicKey;

/// A T=2 Poseidon2 accumulator over each pubkey's `[Xhi, Xlo, Yhi,
/// Ylo]`.
pub fn pubkey_hash(pub_keys: &[VerifyingKey]) -> Result<Fr, InvalidPublicKey> {
    let mut hash = Fr::from(0u64);
    for vk in pub_keys {
        let (x, y) = encoded_xy(vk)?;
        for coordinate in [x, y] {
            let (hi, lo) = coordinate.split_at(16);
            for limb in [hi, lo] {
                hash = poseidon2_compress::<2>(&[hash, limb_to_fr(limb)]);
            }
        }
    }
    Ok(hash)
}

/// Extracts a secp256k1 public key's raw (x, y) coordinates as 32-byte
/// big-endian arrays.
pub(crate) fn encoded_xy(vk: &VerifyingKey) -> Result<([u8; 32], [u8; 32]), InvalidPublicKey> {
    let point = vk.to_sec1_point(false);
    let x = point.x().ok_or(InvalidPublicKey)?;
    let y = point.y().ok_or(InvalidPublicKey)?;
    Ok(((*x).into(), (*y).into()))
}

fn limb_to_fr(limb: &[u8]) -> Fr {
    let mut buf = [0u8; 32];
    buf[16..].copy_from_slice(limb);
    Fr::from_be_bytes_mod_order(&buf)
}
