mod chain;
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
    /// Print the shielded balance of a token for a local account
    Balance {
        /// The local account name to check the balance of
        #[arg(long)]
        account: String,

        /// The ERC20 token address to check the balance of
        #[arg(long)]
        token: Address,
        #[arg(long, env = "TINT_ADDRESS")]
        tint_address: Address,
        #[arg(long, env = "RPC_URL")]
        rpc_url: String,
        #[arg(long, env = "PRIVATE_KEY")]
        private_key: B256,
    },
    /// Shield (deposit) ERC20 funds into a shielded account, auto-approving the transfer
    Shield {
        /// The local account name to shield into
        #[arg(long)]
        to: String,

        /// The ERC20 token address to shield
        #[arg(long)]
        token: Address,

        /// The amount of the token to shield
        #[arg(long)]
        amount: u128,
        #[arg(long, env = "TINT_ADDRESS")]
        tint_address: Address,
        #[arg(long, env = "RPC_URL")]
        rpc_url: String,
        #[arg(long, env = "PRIVATE_KEY")]
        private_key: B256,
    },
    /// Transfer shielded funds to another local account
    Transfer {
        /// The local account name to transfer from
        #[arg(long)]
        from: String,

        /// The local account name to transfer to
        #[arg(long)]
        to: String,

        /// The ERC20 token address to transfer
        #[arg(long)]
        token: Address,

        /// The amount of the token to transfer
        #[arg(long)]
        amount: u128,
        #[arg(long, env = "TINT_ADDRESS")]
        tint_address: Address,
        #[arg(long, env = "RPC_URL")]
        rpc_url: String,
        #[arg(long, env = "PRIVATE_KEY")]
        private_key: B256,
    },
    /// Unshield (withdraw) funds to a plain Ethereum address
    Unshield {
        /// The local account name to unshield from
        #[arg(long)]
        from: String,

        /// The Ethereum address to unshield to
        #[arg(long)]
        to: Address,

        /// The ERC20 token address to unshield
        #[arg(long)]
        token: Address,

        /// The amount of the token to unshield
        #[arg(long)]
        amount: u128,
        #[arg(long, env = "TINT_ADDRESS")]
        tint_address: Address,
        #[arg(long, env = "RPC_URL")]
        rpc_url: String,
        #[arg(long, env = "PRIVATE_KEY")]
        private_key: B256,
    },
    /// Get the Ethereum address for a given private key
    Address {
        /// The private key to derive the address from
        #[arg(long, env = "PRIVATE_KEY")]
        private_key: B256,
    },
    /// Overwrite an address's native balance via Tenderly's setBalance cheatcode (testnets only)
    SetBalance {
        /// The amount of ETH to set the balance to (in wei).
        #[arg(long)]
        amount: u128,

        /// If provided, the address to set the balance of.
        #[arg(long)]
        address: Option<Address>,
        #[arg(long, env = "RPC_URL")]
        rpc_url: String,
        /// If provided, the private key for the address to set the balance of.
        #[arg(long, env = "PRIVATE_KEY")]
        private_key: B256,
    },
    /// Overwrite an address's ERC20 balance via Tenderly's setErc20Balance cheatcode (testnets only)
    SetErc20Balance {
        /// The ERC20 token address to set the balance of.
        #[arg(long)]
        token: Address,

        /// The amount of the token to set the balance to (in wei).
        #[arg(long)]
        amount: u128,

        /// If provided, the address to set the balance of.
        #[arg(long)]
        address: Option<Address>,
        #[arg(long, env = "RPC_URL")]
        rpc_url: String,

        /// If provided, the private key for the address to set the balance of.
        #[arg(long, env = "PRIVATE_KEY")]
        private_key: B256,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        .add_directive("gr1cs=off".parse().unwrap())
        .add_directive("r1cs=off".parse().unwrap());
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let cli = Cli::parse();

    match cli.command {
        Command::ListAccounts => {
            let accounts = config::list_accounts()?;
            for account in accounts {
                tracing::info!("{}", account)
            }
        }
        Command::CreateAccount { name } => {
            config::create_account(&name)?;
            tracing::info!("Created account \"{name}\"");
        }
        Command::Balance {
            account,
            token,
            tint_address,
            rpc_url,
            private_key,
        } => {
            let account = config::load_account(&account)?;
            let session = chain::connect(account, tint_address, &rpc_url, private_key).await?;
            chain::balance(&session, token);
        }
        Command::Shield {
            to,
            token,
            amount,
            tint_address,
            rpc_url,
            private_key,
        } => {
            let to = config::load_account(&to)?;
            let mut session = chain::connect(to, tint_address, &rpc_url, private_key).await?;
            chain::shield(&mut session, token, amount).await?;
        }
        Command::Transfer {
            from,
            to,
            token,
            amount,
            tint_address,
            rpc_url,
            private_key,
        } => {
            let from = config::load_account(&from)?;
            let mut session = chain::connect(from, tint_address, &rpc_url, private_key).await?;
            chain::transfer(&mut session, &to, token, amount).await?;
        }
        Command::Unshield {
            from,
            to,
            token,
            amount,
            tint_address,
            rpc_url,
            private_key,
        } => {
            let from = config::load_account(&from)?;
            let mut session = chain::connect(from, tint_address, &rpc_url, private_key).await?;
            chain::unshield(&mut session, to, token, amount).await?;
        }
        Command::Address { private_key } => {
            let address = chain::eth_address(private_key)?;
            tracing::info!("Ethereum address: {address}");
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
            chain::set_balance(&rpc_url, address, U256::from(amount)).await?;
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
            chain::set_erc20_balance(&rpc_url, token, address, U256::from(amount)).await?;
        }
    }

    Ok(())
}
