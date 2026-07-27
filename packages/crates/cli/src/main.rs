mod chain;
mod circuit;
mod config;

use alloy::primitives::Address;
use clap::{Parser, Subcommand};
use config::Config;

#[derive(Parser)]
#[command(name = "tint-cli", about = "Minimal demo CLI for Tint")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
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
        signaling_server: Option<String>,
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
        signaling_server: Option<String>,
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
        signaling_server: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = Config::load()?;

    match cli.command {
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
            signaling_server,
        } => {
            let account = config::load_account(&account)?;
            let tint_address = config.resolve_tint_address(tint_address)?;
            let signaling_server = config.resolve_signaling_server(signaling_server);
            let mut session = chain::connect(account, tint_address, signaling_server).await?;
            chain::shield(&mut session, token, amount).await?;
        }
        Command::Transfer {
            account,
            to,
            token,
            amount,
            tint_address,
            signaling_server,
        } => {
            let account = config::load_account(&account)?;
            let tint_address = config.resolve_tint_address(tint_address)?;
            let signaling_server = config.resolve_signaling_server(signaling_server);
            let mut session = chain::connect(account, tint_address, signaling_server).await?;
            chain::transfer(&mut session, &to, token, amount).await?;
        }
        Command::Unshield {
            account,
            to,
            token,
            amount,
            tint_address,
            signaling_server,
        } => {
            let account = config::load_account(&account)?;
            let tint_address = config.resolve_tint_address(tint_address)?;
            let signaling_server = config.resolve_signaling_server(signaling_server);
            let mut session = chain::connect(account, tint_address, signaling_server).await?;
            chain::unshield(&mut session, to, token, amount).await?;
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
