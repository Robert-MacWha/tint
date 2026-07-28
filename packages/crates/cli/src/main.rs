mod chain;
mod circuit;
mod config;

use alloy::primitives::{Address, B256, U256};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tint-cli", about = "Minimal demo CLI for Tint")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Lists all local accounts.
    ListAccounts,
    /// Create a new named account
    CreateAccount { name: String },
    /// Print an account's exportable shielded address
    Address { name: String },
    /// Shield (deposit) ERC20 funds into a shielded account, auto-approving the transfer
    Shield {
        #[arg(long)]
        account: String,
        #[arg(long)]
        token: Address,
        #[arg(long)]
        amount: u128,
        #[arg(long)]
        tint_address: Option<Address>,
        #[arg(long)]
        rpc_url: Option<String>,
        #[arg(long)]
        private_key: Option<B256>,
    },
    /// Transfer shielded funds to a local account name or an exported shielded address
    Transfer {
        #[arg(long)]
        account: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        token: Address,
        #[arg(long)]
        amount: u128,
        #[arg(long)]
        tint_address: Option<Address>,
        #[arg(long)]
        rpc_url: Option<String>,
        #[arg(long)]
        private_key: Option<B256>,
    },
    /// Unshield (withdraw) funds to a plain Ethereum address
    Unshield {
        #[arg(long)]
        account: String,
        #[arg(long)]
        to: Address,
        #[arg(long)]
        token: Address,
        #[arg(long)]
        amount: u128,
        #[arg(long)]
        tint_address: Option<Address>,
        #[arg(long)]
        rpc_url: Option<String>,
        #[arg(long)]
        private_key: Option<B256>,
    },
    /// Print the Ethereum address derived from a private key
    EthAddress {
        #[arg(long)]
        private_key: Option<B256>,
    },
    /// Overwrite an address's native balance via Tenderly's setBalance cheatcode (testnets only)
    SetBalance {
        #[arg(long)]
        address: Option<Address>,
        #[arg(long)]
        amount: u128,
        #[arg(long)]
        rpc_url: Option<String>,
        #[arg(long)]
        private_key: Option<B256>,
    },
    /// Overwrite an address's ERC20 balance via Tenderly's setErc20Balance cheatcode (testnets only)
    SetErc20Balance {
        #[arg(long)]
        token: Address,
        #[arg(long)]
        address: Option<Address>,
        #[arg(long)]
        amount: u128,
        #[arg(long)]
        rpc_url: Option<String>,
        #[arg(long)]
        private_key: Option<B256>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::ListAccounts => {
            let accounts = config::list_accounts()?;
            for account in accounts {
                println!("{}", account)
            }
        }
        Command::CreateAccount { name } => {
            let account = config::create_account(&name)?;
            println!("Created account \"{name}\"");
            print_address(&account);
        }
        Command::Address { name } => {
            let account = config::load_account(&name)?;
            print_address(&account);
        }
        Command::Shield {
            account,
            token,
            amount,
            tint_address,
            rpc_url,
            private_key,
        } => {
            let account = config::load_account(&account)?;
            let tint_address = config::resolve_tint_address(tint_address)?;
            let rpc_url = config::resolve_rpc_url(rpc_url)?;
            let private_key = config::resolve_private_key(private_key)?;
            let mut session = chain::connect(account, tint_address, &rpc_url, private_key).await?;
            chain::shield(&mut session, token, amount).await?;
        }
        Command::Transfer {
            account,
            to,
            token,
            amount,
            tint_address,
            rpc_url,
            private_key,
        } => {
            let account = config::load_account(&account)?;
            let tint_address = config::resolve_tint_address(tint_address)?;
            let rpc_url = config::resolve_rpc_url(rpc_url)?;
            let private_key = config::resolve_private_key(private_key)?;
            let mut session = chain::connect(account, tint_address, &rpc_url, private_key).await?;
            chain::transfer(&mut session, &to, token, amount).await?;
        }
        Command::Unshield {
            account,
            to,
            token,
            amount,
            tint_address,
            rpc_url,
            private_key,
        } => {
            let account = config::load_account(&account)?;
            let tint_address = config::resolve_tint_address(tint_address)?;
            let rpc_url = config::resolve_rpc_url(rpc_url)?;
            let private_key = config::resolve_private_key(private_key)?;
            let mut session = chain::connect(account, tint_address, &rpc_url, private_key).await?;
            chain::unshield(&mut session, to, token, amount).await?;
        }
        Command::EthAddress { private_key } => {
            let private_key = config::resolve_private_key(private_key)?;
            println!("{}", chain::eth_address(private_key)?);
        }
        Command::SetBalance {
            address,
            amount,
            rpc_url,
            private_key,
        } => {
            let private_key = config::resolve_private_key(private_key)?;
            let rpc_url = config::resolve_rpc_url(rpc_url)?;
            let address = match address {
                Some(address) => address,
                None => chain::eth_address(private_key)?,
            };
            chain::set_balance(&rpc_url, address, U256::from(amount)).await?;
        }
        Command::SetErc20Balance {
            token,
            address,
            amount,
            rpc_url,
            private_key,
        } => {
            let private_key = config::resolve_private_key(private_key)?;
            let rpc_url = config::resolve_rpc_url(rpc_url)?;
            let address = match address {
                Some(address) => address,
                None => chain::eth_address(private_key)?,
            };
            chain::set_erc20_balance(&rpc_url, token, address, U256::from(amount)).await?;
        }
    }

    Ok(())
}

fn print_address(account: &tint::account::Account) {
    println!(
        "{}",
        alloy::primitives::hex::encode_prefixed(account.receiver().address())
    );
}
