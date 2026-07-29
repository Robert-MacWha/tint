use std::array::repeat;

use ark_bn254::Fr;

use crate::{
    circuit::poseidon2::poseidon2_compress,
    note::{
        commitment::{BaseCommitment, Commitment, SpendableCommitment},
        withdrawal::Withdrawal,
    },
};

#[derive(Clone, Debug)]
pub struct Operation<const N_INPUTS: usize, const N_OUTPUTS: usize, const N_WITHDRAWALS: usize> {
    pub inputs: [SpendableCommitment; N_INPUTS],
    pub output_commitments: [BaseCommitment; N_OUTPUTS],
    pub output_withdrawals: [Withdrawal; N_WITHDRAWALS],
}

impl<const N_INPUTS: usize, const N_OUTPUTS: usize, const N_WITHDRAWALS: usize>
    Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>
{
    pub fn new(
        inputs: [SpendableCommitment; N_INPUTS],
        output_commitments: [BaseCommitment; N_OUTPUTS],
        output_withdrawals: [Withdrawal; N_WITHDRAWALS],
    ) -> Self {
        Operation {
            inputs,
            output_commitments,
            output_withdrawals,
        }
    }

    /// This operation's binding hash, mirroring [`OperationVar::hash`].
    pub fn hash(&self) -> Fr {
        let mut hash = Fr::from(0u64);
        for input in &self.inputs {
            let c = if input.amount_fr() == Fr::from(0u64) {
                Fr::from(0u64)
            } else {
                input.hash()
            };
            hash = poseidon2_compress(&[hash, c]);
        }
        for output in &self.output_commitments {
            let c = if output.amount == 0 {
                Fr::from(0u64)
            } else {
                output.hash()
            };
            hash = poseidon2_compress(&[hash, c]);
        }
        for withdrawal in &self.output_withdrawals {
            let contribution = if withdrawal.amount == 0 {
                Fr::from(0u64)
            } else {
                withdrawal.hash()
            };
            hash = poseidon2_compress(&[hash, contribution]);
        }
        hash
    }
}

impl<const I: usize, const O: usize, const W: usize> Default for Operation<I, O, W> {
    fn default() -> Self {
        Operation {
            inputs: repeat(SpendableCommitment::default()),
            output_commitments: repeat(BaseCommitment::default()),
            output_withdrawals: repeat(Withdrawal::default()),
        }
    }
}
