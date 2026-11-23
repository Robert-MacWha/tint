//! Serves the intent relay.

use std::net::SocketAddr;

use clap::Parser;
use tint_intent_relay::server::router;
use tracing::info;

#[derive(Parser)]
struct Args {
    /// Address to listen on.
    #[arg(long, default_value = "127.0.0.1:8080")]
    listen: SocketAddr,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let listener = tokio::net::TcpListener::bind(args.listen).await?;
    info!("Relay listening on {}", listener.local_addr()?);
    axum::serve(listener, router()).await?;
    Ok(())
}
