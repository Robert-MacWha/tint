use bb_proof_verification::verify_honk_proof;

use input::Input;
use note::Note;
use poseidon::poseidon2::Poseidon2;

pub mod input;
pub mod note;

global MAX_INPUTS: u32 = 4;
global MAX_OUTPUTS: u32 = 4;

global STAGING_VK_HASH: Field = 0; // TODO: Set this to the actual Staging VK Hash in production
global ARCHIVE_VK_HASH: Field = 0; // TODO: Set this to the actual Archive VK Hash in production

fn main(
    // Public Inputs
    staging_root: pub [u8; 32],
    archive_root: pub Field,
    nullifiers: pub [Field; MAX_INPUTS],
    commitments: pub [Field; MAX_OUTPUTS],
    // Private Inputs
    inputs: [Input; MAX_INPUTS],
    outputs: [Note; MAX_OUTPUTS],
) {
    verify_nullifiers(nullifiers, inputs);
    verify_commitments(commitments, outputs);
    verify_balance(inputs, outputs);
    verify_proofs(staging_root, archive_root, nullifiers, inputs);
}

/// Verifies that the computed nullifiers for each input note match the provided nullifiers.
fn verify_nullifiers(nullifiers: [Field; MAX_INPUTS], inputs: [Input; MAX_INPUTS]) {
    for i in 0..MAX_INPUTS {
        let note = inputs[i];
        let computed_nullifier = note.nullifier();
        assert(nullifiers[i] == computed_nullifier, "Nullifier mismatch for input index {i}");
    }
}

/// Verifies that the computed commitments for each output note match the provided commitments.
fn verify_commitments(commitments: [Field; MAX_OUTPUTS], outputs: [Note; MAX_OUTPUTS]) {
    for i in 0..MAX_OUTPUTS {
        let output = outputs[i];
        let computed_commitment = output.commitment();
        assert(commitments[i] == computed_commitment, "Commitment mismatch for output index {i}");
    }
}

/// Verifies that the sum of input amounts equals the sum of output amounts for each asset_id.
fn verify_balance(inputs: [Input; MAX_INPUTS], outputs: [Note; MAX_OUTPUTS]) {
    // Verify for each unique asset_id that the total input amount equals the total output amount
    for i in 0..MAX_INPUTS {
        let current_asset_id = inputs[i].note.asset_id;

        let mut is_first = true;
        for j in 0..i {
            if inputs[j].note.asset_id == current_asset_id {
                is_first = false;
            }
        }

        if is_first {
            let mut input_sum: Field = 0;
            let mut output_sum: Field = 0;

            for j in 0..MAX_INPUTS {
                if inputs[j].note.asset_id == current_asset_id {
                    input_sum += inputs[j].note.amount;
                }
            }

            for k in 0..MAX_OUTPUTS {
                if outputs[k].asset_id == current_asset_id {
                    output_sum += outputs[k].amount;
                }
            }

            assert(input_sum == output_sum, "Asset balance mismatch");
        }
    }

    // Verify that there are no output asset_ids that are not present in the inputs
    for i in 0..MAX_OUTPUTS {
        let current_asset_id = outputs[i].asset_id;

        let mut found_in_input = false;
        for j in 0..MAX_INPUTS {
            if inputs[j].note.asset_id == current_asset_id {
                found_in_input = true;
            }
        }
        assert(found_in_input, "Output asset_id not found in inputs");
    }
}

/// Verifies the spend and inclusion proofs for each input note.
fn verify_proofs(
    staging_root: [u8; 32],
    archive_root: Field,
    nullifiers: [Field; MAX_INPUTS],
    inputs: [Input; MAX_INPUTS],
) {
    for i in 0..MAX_INPUTS {
        let nullifier = nullifiers[i];
        let note = inputs[i];

        let spendability_public_inputs: [Field; 1] = [nullifier];
        verify_honk_proof(
            note.spend_vk,
            note.spend_proof,
            spendability_public_inputs,
            note.note.spend_vk_hash,
        );

        let commitment = note.commitment();
        let commitment_bytes: [u8; 32] = commitment.to_be_bytes::<32>();

        if note.is_archive {
            // Archive Inclusion expects: [root, leaf_commitment]
            let archive_public_inputs: [Field; 2] = [archive_root, commitment];
            verify_honk_proof(
                note.inclusion_vk,
                note.inclusion_proof,
                archive_public_inputs,
                ARCHIVE_VK_HASH,
            );
        } else {
            // Staging Inclusion expects: [root_bytes[32], leaf_commitment[32]]
            let mut staging_public_inputs: [Field; 64] = [0; 64];
            for j in 0..32 {
                staging_public_inputs[32 + j] = staging_root[j] as Field;
                staging_public_inputs[j] = commitment_bytes[j] as Field;
            }
            verify_honk_proof(
                note.inclusion_vk,
                note.inclusion_proof,
                staging_public_inputs,
                STAGING_VK_HASH,
            );
        }
    }
}

#[test]
fn test_verify_nullifiers() {
    let nullifier_privkey: Field = 12345;
    let nullifier_pubkey = Poseidon2::hash([nullifier_privkey], 1);

    let mut inputs = [Input::default(); MAX_INPUTS];
    for i in 0..MAX_INPUTS {
        inputs[i].nullifier_privkey = nullifier_privkey;
        inputs[i].note.nullifier_pubkey = nullifier_pubkey;
        inputs[i].note.salt = i as Field; // distinct notes => distinct nullifiers
    }

    let mut nullifiers = [0; MAX_INPUTS];
    for i in 0..MAX_INPUTS {
        nullifiers[i] = inputs[i].nullifier();
    }

    verify_nullifiers(nullifiers, inputs);
}

#[test(should_fail_with = "Nullifier mismatch")]
fn test_verify_nullifiers_mismatch() {
    let nullifier_privkey: Field = 12345;
    let nullifier_pubkey = Poseidon2::hash([nullifier_privkey], 1);

    let mut inputs = [Input::default(); MAX_INPUTS];
    for i in 0..MAX_INPUTS {
        inputs[i].nullifier_privkey = nullifier_privkey;
        inputs[i].note.nullifier_pubkey = nullifier_pubkey;
    }

    let mut nullifiers = [0; MAX_INPUTS];
    for i in 0..MAX_INPUTS {
        nullifiers[i] = inputs[i].nullifier();
    }
    nullifiers[0] += 1; // corrupt one nullifier

    verify_nullifiers(nullifiers, inputs);
}

#[test]
fn test_verify_commitments() {
    let mut outputs = [Note::default(); MAX_OUTPUTS];
    outputs[0].asset_id = 1;
    outputs[0].amount = 100;
    outputs[1].asset_id = 2;
    outputs[1].amount = 200;

    let mut commitments = [0; MAX_OUTPUTS];
    for i in 0..MAX_OUTPUTS {
        commitments[i] = outputs[i].commitment();
    }

    verify_commitments(commitments, outputs);
}

#[test(should_fail_with = "Commitment mismatch")]
fn test_verify_commitments_mismatch() {
    let mut outputs = [Note::default(); MAX_OUTPUTS];
    outputs[0].asset_id = 1;
    outputs[0].amount = 100;

    let mut commitments = [0; MAX_OUTPUTS];
    for i in 0..MAX_OUTPUTS {
        commitments[i] = outputs[i].commitment();
    }
    commitments[0] += 1; // corrupt one commitment

    verify_commitments(commitments, outputs);
}

#[test]
fn test_verify_balance() {
    let mut inputs = [Input::default(); MAX_INPUTS];
    inputs[0].note.asset_id = 1;
    inputs[0].note.amount = 100;

    let mut outputs = [Note::default(); MAX_OUTPUTS];
    outputs[0].asset_id = 1;
    outputs[0].amount = 100;

    verify_balance(inputs, outputs)
}

#[test]
fn test_verify_balance_multiple_assets() {
    let mut inputs = [Input::default(); MAX_INPUTS];
    inputs[0].note.asset_id = 1;
    inputs[0].note.amount = 100;
    inputs[1].note.asset_id = 2;
    inputs[1].note.amount = 200;

    let mut outputs = [Note::default(); MAX_OUTPUTS];
    outputs[0].asset_id = 1;
    outputs[0].amount = 100;
    outputs[1].asset_id = 2;
    outputs[1].amount = 200;

    verify_balance(inputs, outputs)
}

#[test]
fn test_verify_balance_multiple_notes_same_asset() {
    let mut inputs = [Input::default(); MAX_INPUTS];
    inputs[0].note.asset_id = 1;
    inputs[0].note.amount = 100;
    inputs[1].note.asset_id = 1;
    inputs[1].note.amount = 50;

    let mut outputs = [Note::default(); MAX_OUTPUTS];
    outputs[0].asset_id = 1;
    outputs[0].amount = 150;

    verify_balance(inputs, outputs)
}

#[test(should_fail_with = "Asset balance mismatch")]
fn test_verify_balance_mismatch() {
    let mut inputs = [Input::default(); MAX_INPUTS];
    inputs[0].note.asset_id = 1;
    inputs[0].note.amount = 100;

    let mut outputs = [Note::default(); MAX_OUTPUTS];
    outputs[0].asset_id = 1;
    outputs[0].amount = 90;

    verify_balance(inputs, outputs)
}

#[test(should_fail_with = "Asset balance mismatch")]
fn test_verify_balance_output_asset_not_in_input() {
    let mut inputs = [Input::default(); MAX_INPUTS];
    inputs[0].note.asset_id = 1;
    inputs[0].note.amount = 100;

    let mut outputs = [Note::default(); MAX_OUTPUTS];
    outputs[0].asset_id = 2;
    outputs[0].amount = 100;

    verify_balance(inputs, outputs)
}
