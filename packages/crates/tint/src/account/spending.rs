use alloy_primitives::{Address, Bytes};
use ark_bn254::Fr;

use crate::{
    circuit::join_split::{N_INPUTS, N_OUTPUTS, N_WITHDRAWALS},
    note::commitment::{NullifiableCommitment, SpendableCommitment},
    operation::Operation,
};

/// Accounts that can authorize the spending of their own notes.
///
/// A note is bound to a particular spending account by the spendability hash.
/// Bound notes can only be spent if the contract at their spendability address
/// authorizes spending of the note. Authorization logic may be configured
/// at the time of note creation by specifying a spendability witness.
#[async_trait::async_trait]
pub trait SpendingAccount: std::fmt::Debug {
    fn spendability_address(&self) -> Address;
    fn spendability_witness(&self) -> Fr;

    fn spendability_hash(&self) -> Fr {
        crate::account::spendability_hash(self.spendability_address(), self.spendability_witness())
    }

    /// Converts a nullifiable commitment into a spendable commitment by
    /// generating the `spendability_input`.
    async fn as_spendable(
        &self,
        base: NullifiableCommitment,
        operation: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
    ) -> Result<SpendableCommitment, SpendingAccountError>;
}

/// A noop spending account that does not enforce any spendability rules. Notes
/// with this account type can be spent by anyone that knows the note's encryption
/// and nullifying keys.
#[derive(Clone, Debug, Default)]
pub struct NoopSpendingAccount;

#[derive(Debug, thiserror::Error)]
#[error("spending account error: {inner}")]
pub struct SpendingAccountError {
    inner: Box<dyn std::error::Error + Send + Sync>,
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct Message(String);

#[async_trait::async_trait]
impl SpendingAccount for NoopSpendingAccount {
    fn spendability_address(&self) -> Address {
        Address::ZERO
    }

    fn spendability_witness(&self) -> Fr {
        Fr::from(0)
    }

    async fn as_spendable(
        &self,
        base: NullifiableCommitment,
        _operation: Operation<N_INPUTS, N_OUTPUTS, N_WITHDRAWALS>,
    ) -> Result<SpendableCommitment, SpendingAccountError> {
        Ok(SpendableCommitment::new(
            base.inner.asset,
            base.inner.amount,
            base.nullifier_key,
            self.spendability_address(),
            self.spendability_witness(),
            Bytes::new(),
            base.inner.random,
        ))
    }
}

impl SpendingAccountError {
    pub fn new(inner: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    pub fn msg(s: impl Into<String>) -> Self {
        Self {
            inner: Box::new(Message(s.into())),
        }
    }
}

impl From<String> for SpendingAccountError {
    fn from(s: String) -> Self {
        Self::msg(s)
    }
}

impl From<&str> for SpendingAccountError {
    fn from(s: &str) -> Self {
        Self::msg(s)
    }
}
