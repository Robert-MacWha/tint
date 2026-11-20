#![allow(dead_code)]

use alloy_node_bindings::{Anvil, AnvilInstance};
use alloy_primitives::{Address, B256, U256};
use alloy_provider::{DynProvider, Provider, ProviderBuilder};
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_macro::sol;
use anyhow::Context;
use ark_std::rand::{CryptoRng, RngCore};
use tint::{
    abis::tint::SIMULATION_SLOT,
    account::pub_account::PubAccount,
    deposit::Deposit,
    fr::fr_to_b256,
    note::{asset::AssetId, commitment::Commitment},
    skew_mmr::SkewMmr,
};
use tracing::info;

use crate::common::anvil::{MockToken::MockTokenInstance, Tint::TintInstance};

pub struct Instance {
    // Anvil instance. Kills the anvil process when dropped.
    pub anvil: AnvilInstance,
    pub provider: DynProvider,
    pub tint: TintInstance<DynProvider>,
    pub token: MockTokenInstance<DynProvider>,
}

sol!(
    #[sol(rpc)]
    TintVerifier,
    "../../contracts/out/TintVerifier.sol/TintVerifier.json"
);

sol!(
    #[sol(rpc)]
    Poseidon2T3,
    "../../contracts/out/Poseidon2T3.sol/Poseidon2T3.json"
);

sol!(
    #[sol(rpc)]
    Tint,
    "../../contracts/out/Tint.sol/Tint.json"
);

sol!(
    #[sol(rpc)]
    MockToken,
    "../../contracts/out/MockToken.sol/MockToken.json"
);

impl Instance {
    /// Sets up an anvil instance for testing, deploying tint and a mock ERC20
    pub async fn setup() -> anyhow::Result<Self> {
        let (anvil, provider) = spawn()?;

        let verifier = TintVerifier::deploy(provider.clone())
            .await
            .context("Error deploying verifier")?;
        let poseidon = Poseidon2T3::deploy(provider.clone()).await?;
        let tint = Tint::deploy(provider.clone(), *verifier.address(), *poseidon.address())
            .await
            .context("Error deploying tint")?;

        let token = MockToken::deploy(provider.clone()).await?;

        info!(
            verifier = ?verifier.address(),
            tint = ?tint.address(),
            token = ?token.address(),
            "Deployed contracts",
        );

        Ok(Self {
            anvil,
            provider,
            tint,
            token,
        })
    }

    /// The mock ERC20 as a Tint asset.
    pub fn asset(&self) -> AssetId {
        AssetId::from(*self.token.address())
    }

    /// Deposits `amount` of the mock ERC20 into Tint as a note for `receiver`.
    ///
    /// Returns the deposited note and the deposit's receipt.
    pub async fn deposit<R: RngCore + CryptoRng>(
        &self,
        receiver: PubAccount,
        amount: u128,
        rng: &mut R,
    ) -> anyhow::Result<(Commitment, alloy_rpc_types_eth::TransactionReceipt)> {
        let tint = *self.tint.address();
        let deposit = Deposit::new(tint, receiver, self.asset(), amount, rng);
        send(&self.provider, deposit.approval()).await?;
        let receipt = send(&self.provider, deposit.transaction(rng)?).await?;
        Ok((deposit.note(), receipt))
    }

    /// Returns whether `mmr` matches the note set Tint holds on-chain.
    pub async fn verify_mmr(&self, mmr: &SkewMmr) -> anyhow::Result<bool> {
        let state = mmr.state();
        let frontier = mmr.roots().into_iter().flatten().map(fr_to_b256).collect();
        let verified = self
            .tint
            .verifyFrontier(
                U256::from_le_bytes(*state.packed_ranks()),
                U256::from(state.depth()),
                U256::from(state.count()),
                frontier,
            )
            .call()
            .await?;
        Ok(verified)
    }
}

/// Sends `tx` and waits for a successful receipt.
pub async fn send(
    provider: &impl Provider,
    tx: alloy_rpc_types_eth::TransactionRequest,
) -> anyhow::Result<alloy_rpc_types_eth::TransactionReceipt> {
    let receipt = provider.send_transaction(tx).await?.get_receipt().await?;
    assert!(receipt.status());
    Ok(receipt)
}

/// Lets `tint` accept simulated operations in place of proven ones.
pub async fn enable_simulation(provider: &impl Provider, tint: Address) -> anyhow::Result<()> {
    let _: bool = provider
        .raw_request(
            "anvil_setStorageAt".into(),
            (tint, SIMULATION_SLOT, B256::with_last_byte(1)),
        )
        .await?;
    Ok(())
}

/// Spawns an anvil instance with a provider signing as its first account.
fn spawn() -> anyhow::Result<(AnvilInstance, DynProvider)> {
    let anvil = Anvil::new().spawn();
    let signer = PrivateKeySigner::from_slice(&anvil.first_key().to_bytes())?;
    let provider = ProviderBuilder::new()
        .wallet(signer)
        .connect_http(anvil.endpoint().parse()?)
        .erased();
    Ok((anvil, provider))
}
