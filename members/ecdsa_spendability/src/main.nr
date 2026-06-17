use std::ecdsa_secp256k1::verify_signature;

fn main(pub_key_x: [u8; 32], pub_key_y: [u8; 32], signature: [u8; 64], note_commitment: Field) {
    let message_hash: [u8; 32] = note_commitment.to_be_bytes();
    let is_valid = verify_signature(pub_key_x, pub_key_y, signature, message_hash);
    assert(is_valid == true, "Invalid ECDSA signature for note spend");
}
