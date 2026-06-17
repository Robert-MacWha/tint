global MAX_INPUTS: u32 = 4;
global MAX_OUTPUTS: u32 = 4;

pub struct Input {
    asset_id: Field,
    amount: Field,
    spend_vk_hash: Field,
    salt: Field,
    nullifier_secret: Field,

    spend_vk: [Field; 114],
    spend_proof: [Field; 93],

    inclusion_vk: [Field; 114],
    inclusion_proof: [Field; 93],
    is_archive: bool,
}

pub struct Output {
    asset_id: Field,
    amount: Field,
    spend_vk_hash: Field,
    salt: Field,
}

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
    outputs: [Output; MAX_OUTPUTS],
) {
    verify_commitments(commitments, outputs);
    verify_nullifiers(nullifiers, inputs);
    verify_proofs(staging_root, archive_root, nullifiers, inputs);
    verify_balance(inputs, outputs);
}

/// Verifies that the computed nullifiers for each input note match the provided nullifiers.
fn verify_nullifiers(nullifiers: [Field; MAX_INPUTS], inputs: [Input; MAX_INPUTS]) {
    for i in 0..MAX_INPUTS {
        let note = inputs[i];
        let computed_nullifier = std::hash::poseidon::bn254::hash_4([
            note.asset_id,
            note.amount,
            note.spend_vk_hash,
            note.nullifier_secret,
        ]);
        assert(nullifiers[i] == computed_nullifier, "Nullifier mismatch for input index {i}");
    }
}

/// Verifies that the computed commitments for each output note match the provided commitments.
fn verify_commitments(commitments: [Field; MAX_OUTPUTS], outputs: [Output; MAX_OUTPUTS]) {
    for i in 0..MAX_OUTPUTS {
        let output = outputs[i];
        let computed_commitment = std::hash::poseidon::bn254::hash_4([
            output.asset_id,
            output.amount,
            output.spend_vk_hash,
            output.salt,
        ]);
        assert(commitments[i] == computed_commitment, "Commitment mismatch for output index {i}");
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
        std::verify_proof(
            note.spend_vk,
            note.spend_proof,
            spendability_public_inputs,
            note.spend_vk_hash,
        );

        let commitment_field = std::hash::poseidon::bn254::hash_4([
            note.asset_id,
            note.amount,
            note.spend_vk_hash,
            note.salt,
        ]);
        let commitment_bytes: [u8; 32] = commitment_field.to_be_bytes::<32>();

        if note.is_archive {
            // Archive Inclusion expects: [root, leaf_commitment]
            let archive_public_inputs: [Field; 2] = [archive_root, commitment_field];
            std::verify_proof(
                note.inclusion_vk,
                note.inclusion_proof,
                archive_public_inputs,
                ARCHIVE_VK_HASH,
            );
        } else {
            // Staging Inclusion expects: [root_bytes[32], leaf_bytes[32]]
            let mut staging_public_inputs: [Field; 64] = [0; 64];
            for j in 0..32 {
                staging_public_inputs[32 + j] = staging_root[j] as Field;
                staging_public_inputs[j] = commitment_bytes[j] as Field;
            }
            std::verify_proof(
                note.inclusion_vk,
                note.inclusion_proof,
                staging_public_inputs,
                STAGING_VK_HASH,
            );
        }
    }
}

/// Verifies that the sum of input amounts equals the sum of output amounts for each asset_id.
fn verify_balance(inputs: [Input; MAX_INPUTS], outputs: [Output; MAX_OUTPUTS]) {
    // Verify for each unique asset_id that the total input amount equals the total output amount
    for i in 0..MAX_INPUTS {
        let current_asset_id = inputs[i].asset_id;

        let mut is_first = true;
        for j in 0..i {
            if inputs[j].asset_id == current_asset_id {
                is_first = false;
            }
        }

        if is_first {
            let mut input_sum: Field = 0;
            let mut output_sum: Field = 0;

            for j in 0..MAX_INPUTS {
                if inputs[j].asset_id == current_asset_id {
                    input_sum += inputs[j].amount;
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
            if inputs[j].asset_id == current_asset_id {
                found_in_input = true;
            }
        }
        assert(found_in_input, "Output asset_id not found in inputs");
    }
}

