use alloy::{
    primitives::Address, providers::Provider, rpc::types::TransactionRequest, sol,
    sol_types::SolCall,
};
use alloy_signer_local::PrivateKeySigner;
use anyhow::Context;
use kohaku_userop_kit::{
    builder::UserOperationBuilder,
    bundler::{Bundler, BundlerExt, pimlico::PimlicoBundler},
    smart_account::{SmartAccount, simple_7702_smart_account::Simple7702SmartAccount},
};
use rand_core::OsRng;
use reqwest::Url;
use tint::{account::Account, note::asset::AssetId, operation::OperationBuilder};
use tint_paymaster::{
    OperationBuilderPaymasterExt, Refund, TintPaymaster, UserOperationPaymasterExt,
};
use tracing::info;

use crate::{
    account::Authorizer,
    chain::{self, Chain},
};

sol! {
    function WETH() external view returns (address);
}

/// Options for paying gas from shielded WETH through a `TintPaymaster`.
#[derive(clap::Args)]
pub struct SponsorArgs {
    /// Pay gas from shielded WETH through the `TintPaymaster` and an ERC-4337 bundler
    #[arg(long)]
    sponsor: bool,
    #[arg(long, env = "PAYMASTER_ADDRESS")]
    paymaster_address: Option<Address>,
    #[arg(long, env = "BUNDLER_URL")]
    bundler_url: Option<Url>,
    /// Most shielded WETH, in base units, to set aside for the paymaster fee
    #[arg(long, default_value_t = 10u128.pow(17))]
    max_fee: u128,
}

/// A `TintPaymaster` and the bundler its userops go through.
pub struct Sponsor {
    paymaster: TintPaymaster,
    bundler: PimlicoBundler,
    max_fee: u128,
}

impl Sponsor {
    /// Looks up the paymaster's fee token, if `args` asks for sponsorship.
    pub async fn connect(chain: &Chain, args: &SponsorArgs) -> anyhow::Result<Option<Self>> {
        if !args.sponsor {
            return Ok(None);
        }
        let address = args
            .paymaster_address
            .context("--sponsor needs PAYMASTER_ADDRESS")?;
        let url = args
            .bundler_url
            .clone()
            .context("--sponsor needs BUNDLER_URL")?;

        let call = TransactionRequest::default()
            .to(address)
            .input(WETHCall {}.abi_encode().into());
        let weth = WETHCall::abi_decode_returns(&chain.provider.call(call).await?)?;
        Ok(Some(Self {
            paymaster: TintPaymaster { address, weth },
            bundler: PimlicoBundler::new(url),
            max_fee: args.max_fee,
        }))
    }

    /// The WETH an operation must spend to cover the largest fee allowed.
    pub fn fee_budget(&self) -> (AssetId, u128) {
        (AssetId::from(self.paymaster.weth), self.max_fee)
    }

    /// Submits `builder`'s operation in a userop from a fresh EIP-7702 account,
    /// paying its fee from the operation and refunding the rest as a note.
    pub async fn submit(
        &self,
        chain: &Chain,
        account: &Account,
        authorizer: &Authorizer,
        builder: OperationBuilder,
    ) -> anyhow::Result<()> {
        let owner = PrivateKeySigner::random();
        let smart_account =
            Simple7702SmartAccount::new(chain.provider.clone(), owner.address(), chain.id);
        let userop = UserOperationBuilder::new_with_smart_account(&smart_account).await?;
        let refund = Refund::deposit(&account.pub_account(), &mut OsRng)?;
        let mut payment = self.paymaster.pay(1, refund, smart_account.address());

        info!("Estimating gas with a simulated operation...");
        let mut simulated = builder.clone().with_paymaster(&payment).build(&mut OsRng)?;
        authorizer
            .authorize_simulation(&mut simulated, chain.now().await?)
            .await?;
        let simulated = simulated.simulate_with_mmr(&chain.mmr)?;
        let overrides = self.paymaster.simulation_override(chain.tint);
        let userop = userop
            .with_tint_paymaster(&payment, &simulated)?
            .with_gas_estimate(&self.bundler, Some(&overrides))
            .await?;

        payment.fee = userop.max_gas_cost();
        anyhow::ensure!(
            payment.fee <= self.max_fee,
            "fee {} exceeds --max-fee {}",
            payment.fee,
            self.max_fee
        );
        let op = builder.with_paymaster(&payment).build(&mut OsRng)?;
        let proven = chain::prove(chain, authorizer, op).await?;
        let userop = userop
            .with_tint_paymaster(&payment, &proven)?
            .build()
            .sign(&owner)
            .await?;

        info!("Sending userop paying up to {} WETH...", payment.fee);
        let hash = self.bundler.send_user_operation(&userop).await?;
        let receipt = self.bundler.wait_for_receipt(hash).await?;
        anyhow::ensure!(receipt.success, "userop {} failed", receipt.user_op_hash);
        info!(
            "userop {} included in tx {} (gas cost {})",
            receipt.user_op_hash, receipt.receipt.transaction_hash, receipt.actual_gas_cost
        );
        Ok(())
    }
}
