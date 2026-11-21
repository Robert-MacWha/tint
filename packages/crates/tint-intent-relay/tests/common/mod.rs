#![allow(dead_code)]

#[path = "../../../tint/tests/common/anvil.rs"]
pub mod anvil;
#[path = "../../../tint-intent-policy/tests/common/policy.rs"]
pub mod policy;
#[path = "../../../tint/tests/common/util.rs"]
pub mod util;

use reqwest::Url;
use tint_intent_relay::{client::RelayClient, server::router};

/// Serves a fresh relay on a random local port, returning its client.
pub async fn spawn_relay() -> anyhow::Result<RelayClient> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let url = Url::parse(&format!("http://{}", listener.local_addr()?))?;
    tokio::spawn(async move { axum::serve(listener, router()).await });
    Ok(RelayClient::new(url))
}
