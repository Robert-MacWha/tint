mod account;
mod adapter;
mod address;
mod chain;
mod config;
mod intent;
mod paymaster;

use std::{fs, path::PathBuf};

use alloy::primitives::{Address, B256, Bytes, U256, address, aliases::U24};
use anyhow::Context;
use clap::{Parser, Subcommand};
use tint_adapter::Call;

use crate::{
    account::PolicyArgs,
    chain::{Chain, ChainArgs},
    config::{SpendPolicy, StoredAccount},
    intent::IntentArgs,
    paymaster::SponsorArgs,
};

/// Uniswap v3 `SwapRouter02` on mainnet.
const SWAP_ROUTER: Address = address!("0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45");
/// Aave v3 core `Pool` on mainnet.
const AAVE_POOL: Address = address!("0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2");

#[derive(Parser)]
#[command(name = "tint-cli", about = "Command-line wallet for Tint")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create, import, export and list local accounts
    #[command(subcommand)]
    Account(AccountCommand),
    /// Print an account's receiving address, to paste as a recipient
    Address { account: String },
    /// Sync the note set into the local snapshot
    Sync {
        #[command(flatten)]
        chain: ChainArgs,
    },
    /// Print an account's shielded balances
    Balance {
        account: String,
        /// Only print this token's balance
        #[arg(long)]
        token: Option<Address>,
        #[command(flatten)]
        chain: ChainArgs,
    },
    /// Shield (deposit) ERC20 funds, auto-approving the transfer
    Shield {
        /// Local account name or `tint:` address to shield into
        to: String,
        token: Address,
        amount: u128,
        #[command(flatten)]
        chain: ChainArgs,
    },
    /// Transfer shielded funds
    Transfer {
        from: String,
        /// Local account name or `tint:` address to transfer to
        to: String,
        token: Address,
        amount: u128,
        #[command(flatten)]
        sponsor: SponsorArgs,
        #[command(flatten)]
        chain: ChainArgs,
    },
    /// Unshield (withdraw) funds to a plain Ethereum address
    Unshield {
        from: String,
        to: Address,
        token: Address,
        amount: u128,
        #[command(flatten)]
        sponsor: SponsorArgs,
        #[command(flatten)]
        chain: ChainArgs,
    },
    /// Relay shielded funds through the `TintAdapter` to arbitrary calls
    Relay {
        from: String,
        /// Funds to unshield to the adapter, as `TOKEN=AMOUNT`. Repeatable.
        #[arg(long = "input", value_parser = parse_amount)]
        inputs: Vec<(Address, u128)>,
        /// A call for the adapter to make, as `TARGET=CALLDATA`. Repeatable.
        #[arg(long = "call", value_parser = parse_call)]
        calls: Vec<Call>,
        /// A token whose adapter balance is shielded back afterwards. Repeatable.
        #[arg(long = "shield")]
        shields: Vec<Address>,
        #[command(flatten)]
        adapter: AdapterArgs,
        #[command(flatten)]
        chain: ChainArgs,
    },
    /// Swap shielded funds on Uniswap v3 through the adapter
    Swap {
        from: String,
        token_in: Address,
        token_out: Address,
        amount: u128,
        /// Least `token_out` to accept, in base units
        #[arg(long)]
        min_out: u128,
        /// Pool fee tier, in hundredths of a bip
        #[arg(long, default_value_t = U24::from(500))]
        pool_fee: U24,
        #[arg(long, default_value_t = SWAP_ROUTER)]
        router: Address,
        #[command(flatten)]
        adapter: AdapterArgs,
        #[command(flatten)]
        chain: ChainArgs,
    },
    /// Supply shielded funds to Aave v3 through the adapter, shielding the aTokens
    AaveSupply {
        from: String,
        token: Address,
        amount: u128,
        /// The aToken Aave mints for `token`
        #[arg(long)]
        a_token: Address,
        #[arg(long, default_value_t = AAVE_POOL)]
        pool: Address,
        #[command(flatten)]
        adapter: AdapterArgs,
        #[command(flatten)]
        chain: ChainArgs,
    },
    /// Post, fill, claim and cancel shielded intents
    #[command(subcommand)]
    Intent(IntentCommand),
    /// Get the Ethereum address for a given private key
    EthAddress {
        #[arg(long, env = "PRIVATE_KEY")]
        private_key: B256,
    },
    /// Overwrite an address's native balance via Tenderly's setBalance cheatcode
    /// (testnets only)
    SetBalance {
        /// The amount of ETH to set the balance to (in wei).
        amount: u128,
        /// If provided, the address to set the balance of.
        address: Option<Address>,
        #[arg(long, env = "RPC_URL")]
        rpc_url: String,
        /// If provided, the private key for the address to set the balance of.
        #[arg(long, env = "PRIVATE_KEY")]
        private_key: B256,
    },
    /// Overwrite an address's ERC20 balance via Tenderly's setErc20Balance cheatcode
    /// (testnets only)
    SetErc20Balance {
        /// The ERC20 token address to set the balance of.
        token: Address,
        /// The amount of the token to set the balance to (in wei).
        amount: u128,
        /// If provided, the address to set the balance of.
        address: Option<Address>,
        #[arg(long, env = "RPC_URL")]
        rpc_url: String,
        /// If provided, the private key for the address to set the balance of.
        #[arg(long, env = "PRIVATE_KEY")]
        private_key: B256,
    },
}

#[derive(Subcommand)]
enum AccountCommand {
    /// Create a new account with a random seed
    Create {
        name: String,
        #[command(flatten)]
        policy: PolicyArgs,
    },
    /// Import an account file written by `account export`
    Import { name: String, file: PathBuf },
    /// Print an account file, including its seed and policy secrets
    Export { name: String },
    /// List local accounts
    List,
}

#[derive(Subcommand)]
enum IntentCommand {
    /// Fund an intent note offering `offer` for `want`, and post it to the relay
    Post {
        account: String,
        /// The offered funds, as `TOKEN=AMOUNT`
        #[arg(long, value_parser = parse_amount)]
        offer: (Address, u128),
        /// The wanted funds, as `TOKEN=AMOUNT`
        #[arg(long, value_parser = parse_amount)]
        want: (Address, u128),
        /// Seconds until the intent expires
        #[arg(long, default_value_t = 3600)]
        expires_in: u64,
        #[command(flatten)]
        intent: IntentArgs,
        #[command(flatten)]
        chain: ChainArgs,
    },
    /// List the relay's open intents
    List {
        #[arg(long)]
        offer_asset: Option<Address>,
        #[arg(long)]
        want_asset: Option<Address>,
        #[command(flatten)]
        intent: IntentArgs,
    },
    /// Print the state of an account's posted intents
    Status {
        account: String,
        #[command(flatten)]
        chain: ChainArgs,
    },
    /// Move a filled intent's received note into the account
    Claim {
        account: String,
        id: B256,
        #[command(flatten)]
        chain: ChainArgs,
    },
    /// Reclaim an unfilled intent's note and remove it from the relay
    Cancel {
        account: String,
        id: B256,
        #[command(flatten)]
        intent: IntentArgs,
        #[command(flatten)]
        chain: ChainArgs,
    },
    /// Fill a relay intent from the account's notes
    Fill {
        account: String,
        id: B256,
        #[command(flatten)]
        intent: IntentArgs,
        #[command(flatten)]
        chain: ChainArgs,
    },
}

#[derive(clap::Args)]
struct AdapterArgs {
    #[arg(long, env = "ADAPTER_ADDRESS")]
    adapter_address: Address,
}

#[tokio::main]
#[expect(clippy::too_many_lines, reason = "one arm per command")]
async fn main() -> anyhow::Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        .add_directive("gr1cs=off".parse()?)
        .add_directive("r1cs=off".parse()?);
    tracing_subscriber::fmt().with_env_filter(filter).init();

    match Cli::parse().command {
        Command::Account(command) => run_account(command).await,
        Command::Address { account } => {
            let pub_account = config::load_account(&account)?.account().pub_account();
            println!("{}", address::encode(&pub_account));
            Ok(())
        }
        Command::Sync { chain } => {
            let chain = Chain::connect(&chain).await?;
            println!("Synced to block {}", chain.snapshot.range.end);
            Ok(())
        }
        Command::Balance {
            account,
            token,
            chain,
        } => {
            let account = config::load_account(&account)?.account();
            let chain = Chain::connect(&chain).await?;
            for (asset, amount) in chain::balances(&chain, &account) {
                if token.is_none_or(|token| asset == token) {
                    println!("{asset}: {amount}");
                }
            }
            Ok(())
        }
        Command::Shield {
            to,
            token,
            amount,
            chain,
        } => {
            let to = address::parse_recipient(&to)?;
            let mut chain = Chain::connect(&chain).await?;
            chain::shield(&mut chain, to, token, amount).await
        }
        Command::Transfer {
            from,
            to,
            token,
            amount,
            sponsor,
            chain,
        } => {
            let from = config::load_account(&from)?;
            let to = address::parse_recipient(&to)?;
            let mut chain = Chain::connect(&chain).await?;
            chain::transfer(&mut chain, &from, to, token, amount, &sponsor).await
        }
        Command::Unshield {
            from,
            to,
            token,
            amount,
            sponsor,
            chain,
        } => {
            let from = config::load_account(&from)?;
            let mut chain = Chain::connect(&chain).await?;
            chain::unshield(&mut chain, &from, to, token, amount, &sponsor).await
        }
        Command::Relay {
            from,
            inputs,
            calls,
            shields,
            adapter,
            chain,
        } => {
            let from = config::load_account(&from)?;
            let mut chain = Chain::connect(&chain).await?;
            let adapter = adapter.adapter_address;
            adapter::relay(&mut chain, &from, adapter, inputs, calls, &shields).await
        }
        Command::Swap {
            from,
            token_in,
            token_out,
            amount,
            min_out,
            pool_fee,
            router,
            adapter,
            chain,
        } => {
            let from = config::load_account(&from)?;
            let mut chain = Chain::connect(&chain).await?;
            let adapter = adapter.adapter_address;
            let tokens = (token_in, token_out);
            let calls = adapter::swap_calls(adapter, router, tokens, amount, pool_fee, min_out);
            let inputs = vec![(token_in, amount)];
            adapter::relay(&mut chain, &from, adapter, inputs, calls, &[token_out]).await
        }
        Command::AaveSupply {
            from,
            token,
            amount,
            a_token,
            pool,
            adapter,
            chain,
        } => {
            let from = config::load_account(&from)?;
            let mut chain = Chain::connect(&chain).await?;
            let adapter = adapter.adapter_address;
            let calls = adapter::aave_supply_calls(adapter, pool, token, amount);
            let inputs = vec![(token, amount)];
            adapter::relay(&mut chain, &from, adapter, inputs, calls, &[a_token]).await
        }
        Command::Intent(command) => Box::pin(run_intent(command)).await,
        Command::EthAddress { private_key } => {
            println!("{}", chain::eth_address(private_key)?);
            Ok(())
        }
        Command::SetBalance {
            address,
            amount,
            rpc_url,
            private_key,
        } => {
            let address = match address {
                Some(address) => address,
                None => chain::eth_address(private_key)?,
            };
            chain::set_balance(&rpc_url, address, U256::from(amount)).await
        }
        Command::SetErc20Balance {
            token,
            address,
            amount,
            rpc_url,
            private_key,
        } => {
            let address = match address {
                Some(address) => address,
                None => chain::eth_address(private_key)?,
            };
            chain::set_erc20_balance(&rpc_url, token, address, U256::from(amount)).await
        }
    }
}

async fn run_account(command: AccountCommand) -> anyhow::Result<()> {
    match command {
        AccountCommand::Create { name, policy } => {
            let account = account::create(&policy).await?;
            config::save_new_account(&name, &account)?;
            println!("{}", address::encode(&account.account().pub_account()));
        }
        AccountCommand::Import { name, file } => {
            let contents =
                fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
            let account: StoredAccount =
                serde_json::from_str(&contents).context("not a tint-cli account file")?;
            config::save_new_account(&name, &account)?;
        }
        AccountCommand::Export { name } => {
            let account = config::load_account(&name)?;
            println!("{}", serde_json::to_string_pretty(&account)?);
        }
        AccountCommand::List => {
            for name in config::list_accounts()? {
                let account = config::load_account(&name)?;
                println!("{name}: {}", policy_name(&account.spend));
            }
        }
    }
    Ok(())
}

async fn run_intent(command: IntentCommand) -> anyhow::Result<()> {
    match command {
        IntentCommand::Post {
            account,
            offer,
            want,
            expires_in,
            intent,
            chain,
        } => {
            let mut chain = Chain::connect(&chain).await?;
            intent::post(&mut chain, &account, &intent, offer, want, expires_in).await
        }
        IntentCommand::List {
            offer_asset,
            want_asset,
            intent,
        } => intent::list(&intent, offer_asset, want_asset).await,
        IntentCommand::Status { account, chain } => {
            let chain = Chain::connect(&chain).await?;
            intent::status(&chain, &account).await
        }
        IntentCommand::Claim { account, id, chain } => {
            let mut chain = Chain::connect(&chain).await?;
            intent::claim(&mut chain, &account, id).await
        }
        IntentCommand::Cancel {
            account,
            id,
            intent,
            chain,
        } => {
            let mut chain = Chain::connect(&chain).await?;
            intent::cancel(&mut chain, &account, &intent, id).await
        }
        IntentCommand::Fill {
            account,
            id,
            intent,
            chain,
        } => {
            let mut chain = Chain::connect(&chain).await?;
            intent::fill(&mut chain, &account, &intent, id).await
        }
    }
}

fn policy_name(spend: &SpendPolicy) -> &'static str {
    match spend {
        SpendPolicy::None => "none",
        SpendPolicy::Password => "password",
        SpendPolicy::Multisig { .. } => "multisig",
        SpendPolicy::HardwareWallet(_) => "hardware-wallet",
        SpendPolicy::Timelock { .. } => "timelock",
    }
}

/// Parses `TOKEN=AMOUNT`.
fn parse_amount(s: &str) -> anyhow::Result<(Address, u128)> {
    let (token, amount) = s.split_once('=').context("expected TOKEN=AMOUNT")?;
    Ok((token.parse()?, amount.parse()?))
}

/// Parses `TARGET=CALLDATA`, with hex calldata.
fn parse_call(s: &str) -> anyhow::Result<Call> {
    let (target, data) = s.split_once('=').context("expected TARGET=CALLDATA")?;
    Ok(Call::new(target.parse()?, data.parse::<Bytes>()?))
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn cli_is_well_formed() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_relay_inputs_and_calls() {
        let cli = Cli::try_parse_from([
            "tint-cli",
            "relay",
            "alice",
            "--input",
            "0x00000000000000000000000000000000000000aa=100",
            "--call",
            "0x00000000000000000000000000000000000000bb=0x1234",
            "--shield",
            "0x00000000000000000000000000000000000000cc",
            "--adapter-address",
            "0x00000000000000000000000000000000000000dd",
            "--rpc-url",
            "http://localhost:8545",
            "--tint-address",
            "0x00000000000000000000000000000000000000ee",
        ])
        .unwrap();

        let Command::Relay { inputs, calls, .. } = cli.command else {
            panic!("expected a relay command");
        };
        assert_eq!(inputs, vec![(Address::with_last_byte(0xaa), 100)]);
        assert_eq!(calls[0].target, Address::with_last_byte(0xbb));
        assert_eq!(calls[0].data, Bytes::from(vec![0x12, 0x34]));
    }
}
