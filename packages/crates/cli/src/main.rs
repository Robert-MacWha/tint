use alloy::{
    network::TransactionBuilder,
    primitives::{address, utils::parse_ether},
    providers::{Provider, ProviderBuilder},
    rpc::{client::RpcClient, types::TransactionRequest},
};
use openlv::{SignalingProtocol, provider::rpc_client};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    let session = openlv::dapp()
        .protocol(SignalingProtocol::Mqtt)
        .server("wss://test.mosquitto.org:8081/mqtt")
        .on_request(|msg| async move {
            println!("Received request: {:?}", msg);
            Ok(json!({"result": "success"}))
        })
        .await?;

    session.connect().await?;
    println!("Session uri: {}", session.uri());
    println!("Waiting for OpenLV to link with the dapp...");
    session.wait_for_link().await?;
    println!("Linked with the dapp!");

    let client = rpc_client(session);
    let provider = ProviderBuilder::new().connect_client(client);

    let accounts = provider.get_accounts().await?;
    let from = accounts
        .get(0)
        .ok_or_else(|| anyhow::anyhow!("No accounts available"))?;
    println!("Accounts: {:?}", accounts);

    let tx = TransactionRequest::default()
        .with_from(*from)
        .with_to(address!("000000000000000000000000000000000000dEaD"))
        .with_value(parse_ether("0")?);

    println!("Sending transaction: {:?}", tx);
    let pending = provider.send_transaction(tx).await?;
    println!("pending tx: {pending:?}");

    Ok(())
}
