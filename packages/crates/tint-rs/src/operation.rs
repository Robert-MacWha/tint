use std::array::repeat;

use crate::note::{
    commitment::{Commitment, SpendableCommitment},
    withdrawal::Withdrawal,
};

#[derive(Clone, Debug)]
pub struct Operation<const N_INPUTS: usize, const N_OUTPUTS: usize, const N_WITHDRAWALS: usize> {
    pub inputs: [SpendableCommitment; N_INPUTS],
    pub output_commitments: [Commitment; N_OUTPUTS],
    pub output_withdrawals: [Withdrawal; N_WITHDRAWALS],
}

impl<const N_INPUTS: usize, const N_OUTPUTS: usize, const N_WITHDRAWALS: usize>
    Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>
{
    pub fn new(
        inputs: [SpendableCommitment; N_INPUTS],
        output_commitments: [Commitment; N_OUTPUTS],
        output_withdrawals: [Withdrawal; N_WITHDRAWALS],
    ) -> Self {
        Operation {
            inputs,
            output_commitments,
            output_withdrawals,
        }
    }
}

impl<const I: usize, const O: usize, const W: usize> Default for Operation<I, O, W> {
    fn default() -> Self {
        Operation {
            inputs: repeat(SpendableCommitment::default()),
            output_commitments: repeat(Commitment::default()),
            output_withdrawals: repeat(Withdrawal::default()),
        }
    }
}
