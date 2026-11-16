//! Sends operations as ERC-4337 userops whose gas a `TintPaymaster` takes
//! from the operation's notes.

use alloy::{
    primitives::B256,
    providers::{DynProvider, Provider},
    rpc::{
        client::{ClientBuilder, RpcClient},
        types::state::StateOverride,
    },
    signers::local::PrivateKeySigner,
    sol,
};
use anyhow::Context;
use ark_bn254::Bn254;
use kohaku_userop_kit::{
    builder::UserOperationBuilder,
    signable_user_operation::SignableUserOperation,
    signed_user_operation::SignedUserOperation,
    smart_account::{SmartAccount, simple_7702_smart_account::Simple7702SmartAccount},
    user_operation::{UserOperationGasEstimate, UserOperationReceipt},
};
use rand_core::OsRng;
use serde::Deserialize;
use tint::{operation::OperationBuilder, skew_mmr::SkewMmr};
use tint_groth16::artifacts::Artifacts;
use tint_paymaster::{
    OperationBuilderPaymasterExt, Refund, TintPaymaster, UserOperationPaymasterExt,
};
use tint_web_protocol::Sponsor;
use wasm_bindgen_futures::JsFuture;

use crate::{handler::Session, progress, scope};

/// How often, and how many times, to poll the bundler for a userop's receipt.
const RECEIPT_POLL_MS: i32 = 2_000;
const RECEIPT_POLLS: u32 = 60;

sol! {
    #[sol(rpc)]
    interface ITintPaymaster {
        function WETH() external view returns (address);
    }
}

/// A bundler's ERC-4337 JSON-RPC endpoint.
struct Bundler(RpcClient);

/// Gas prices from alto's `pimlico_getUserOperationGasPrice`.
#[derive(Debug, Deserialize)]
struct GasPrices {
    standard: GasPrice,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GasPrice {
    #[serde(with = "alloy::serde::quantity")]
    max_fee_per_gas: u128,
    #[serde(with = "alloy::serde::quantity")]
    max_priority_fee_per_gas: u128,
}

/// The gas limits from `eth_estimateUserOperationGas`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GasLimits {
    #[serde(with = "alloy::serde::quantity")]
    call_gas_limit: u128,
    #[serde(with = "alloy::serde::quantity")]
    verification_gas_limit: u128,
    #[serde(with = "alloy::serde::quantity")]
    pre_verification_gas: u128,
    #[serde(default, with = "alloy::serde::quantity::opt")]
    paymaster_verification_gas_limit: Option<u128>,
    #[serde(default, with = "alloy::serde::quantity::opt")]
    paymaster_post_op_gas_limit: Option<u128>,
}

/// Proves `operation` paying the paymaster its userop's maximum gas cost,
/// refunding what's left as a note to `session`'s account, and sends it
/// through the bundler from a fresh 7702 account.
pub async fn send(
    id: u64,
    sponsor: &Sponsor,
    paymaster: TintPaymaster,
    session: &Session,
    operation: OperationBuilder,
    mmr: &SkewMmr,
    artifacts: &Artifacts<Bn254>,
) -> anyhow::Result<()> {
    let provider = &session.provider;
    let bundler = Bundler::new(&sponsor.bundler_url)?;

    // A throwaway sender, so the userop isn't linked to anything else.
    let owner = PrivateKeySigner::random();
    let chain_id = provider.get_chain_id().await?;
    let account = Simple7702SmartAccount::new(provider.clone(), owner.address(), chain_id);
    let builder = UserOperationBuilder::new_with_smart_account(&account)
        .await?
        .with_call(&Vec::new());
    let refund = Refund::deposit(&session.account.pub_account(), &mut OsRng)?;
    let mut payment = paymaster.pay(1, refund, account.address());

    // Estimate gas with a simulated operation paying a nominal fee.
    progress(id, "Estimating userop gas");
    let simulated = operation
        .clone()
        .with_paymaster(&payment)
        .build(&mut OsRng)
        .context("the notes don't cover the amount, or aren't WETH")?
        .simulate_with_mmr(mmr)?;
    let builder = builder.with_tint_paymaster(&payment, &simulated)?;
    let gas = bundler
        .estimate_gas(
            &builder.build(),
            &paymaster.simulation_override(session.tint),
        )
        .await?;
    let builder = builder.with_gas(gas);

    payment.fee = builder.max_gas_cost();
    progress(id, &format!("Proving, paying a {} wei fee", payment.fee));
    let proven = operation
        .with_paymaster(&payment)
        .build(&mut OsRng)
        .with_context(|| {
            format!(
                "the notes don't cover the amount plus the {} wei fee",
                payment.fee
            )
        })?
        .prove_with_mmr(mmr, artifacts, &mut OsRng)?;
    let userop = builder
        .with_tint_paymaster(&payment, &proven)?
        .build()
        .sign(&owner)
        .await?;

    progress(id, "Sending userop");
    let hash = bundler.send(&userop).await?;
    progress(id, &format!("Waiting for userop {hash}"));
    let receipt = bundler.wait_for_receipt(hash).await?;
    if !receipt.success {
        anyhow::bail!(
            "userop {hash} reverted in transaction {}",
            receipt.receipt.transaction_hash
        );
    }
    Ok(())
}

/// `sponsor`'s paymaster, with the WETH it takes its fee in.
pub async fn paymaster(provider: &DynProvider, sponsor: &Sponsor) -> anyhow::Result<TintPaymaster> {
    let weth = ITintPaymaster::new(sponsor.paymaster, provider)
        .WETH()
        .call()
        .await
        .context("reading the paymaster's WETH")?;
    Ok(TintPaymaster {
        address: sponsor.paymaster,
        weth,
    })
}

impl Bundler {
    fn new(url: &str) -> anyhow::Result<Self> {
        let url = url.parse().context("invalid bundler URL")?;
        Ok(Self(ClientBuilder::default().http(url)))
    }

    async fn estimate_gas(
        &self,
        op: &SignableUserOperation,
        state_override: &StateOverride,
    ) -> anyhow::Result<UserOperationGasEstimate> {
        let limits: GasLimits = self
            .0
            .request(
                "eth_estimateUserOperationGas",
                (&op.user_op, op.entry_point, state_override),
            )
            .await
            .context("estimating userop gas")?;
        let prices: GasPrices = self
            .0
            .request_noparams("pimlico_getUserOperationGasPrice")
            .await
            .context("fetching userop gas prices")?;

        Ok(UserOperationGasEstimate {
            pre_verification_gas: limits.pre_verification_gas,
            verification_gas_limit: limits.verification_gas_limit,
            call_gas_limit: limits.call_gas_limit,
            paymaster_verification_gas_limit: limits.paymaster_verification_gas_limit,
            paymaster_post_op_gas_limit: limits.paymaster_post_op_gas_limit,
            max_fee_per_gas: prices.standard.max_fee_per_gas,
            max_priority_fee_per_gas: prices.standard.max_priority_fee_per_gas,
        })
    }

    async fn send(&self, op: &SignedUserOperation) -> anyhow::Result<B256> {
        self.0
            .request("eth_sendUserOperation", (&op.user_op, op.entry_point))
            .await
            .context("sending userop")
    }

    async fn wait_for_receipt(&self, hash: B256) -> anyhow::Result<UserOperationReceipt> {
        for _ in 0..RECEIPT_POLLS {
            let receipt: Option<UserOperationReceipt> = self
                .0
                .request("eth_getUserOperationReceipt", (hash,))
                .await
                .context("fetching userop receipt")?;
            if let Some(receipt) = receipt {
                return Ok(receipt);
            }
            sleep(RECEIPT_POLL_MS).await;
        }
        anyhow::bail!("userop {hash} wasn't included in time")
    }
}

async fn sleep(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let _ = scope().set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
    });
    let _ = JsFuture::from(promise).await;
}
