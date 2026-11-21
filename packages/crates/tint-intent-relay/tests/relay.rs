mod common;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use alloy_primitives::{Address, B256};
use ark_std::rand::{SeedableRng, rngs::StdRng};
use tint::{account::Account, note::asset::AssetId, policy::Policy};
use tint_intent_policy::intent::{CancelKey, Intent};
use tint_intent_relay::server::IntentFilter;

use crate::common::spawn_relay;

const A: Address = Address::repeat_byte(1);
const B: Address = Address::repeat_byte(2);

/// Should list posted intents in order, filtered by asset.
#[tokio::test]
async fn post_and_list() -> anyhow::Result<()> {
    let relay = spawn_relay().await?;
    let (a_for_b, _) = intent(A, B, now() + 60)?;
    let (b_for_a, _) = intent(B, A, now() + 60)?;

    assert_eq!(relay.post(&a_for_b).await?, a_for_b.id());
    relay.post(&b_for_a).await?;

    let all = relay.list(&IntentFilter::default()).await?;
    assert_eq!(all, vec![a_for_b.clone(), b_for_a.clone()]);

    let filter = IntentFilter {
        offer_asset: Some(B),
        want_asset: None,
    };
    assert_eq!(relay.list(&filter).await?, vec![b_for_a.clone()]);

    let filter = IntentFilter {
        offer_asset: None,
        want_asset: Some(B),
    };
    assert_eq!(relay.list(&filter).await?, vec![a_for_b]);
    Ok(())
}

/// Should reject intents whose offer isn't bound to their terms.
#[tokio::test]
async fn rejects_invalid() -> anyhow::Result<()> {
    let relay = spawn_relay().await?;
    let (mut intent, _) = intent(A, B, now() + 60)?;
    intent.want.amount += 1;

    assert!(relay.post(&intent).await.is_err());
    Ok(())
}

/// Should reject expired intents, and stop listing intents once they expire.
#[tokio::test]
async fn expires() -> anyhow::Result<()> {
    let relay = spawn_relay().await?;
    let (expired, _) = intent(A, B, now() - 1)?;
    let (expiring, _) = intent(A, B, now() + 1)?;

    assert!(relay.post(&expired).await.is_err());
    relay.post(&expiring).await?;

    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(relay.list(&IntentFilter::default()).await?.is_empty());
    Ok(())
}

/// Should remove an intent only given its cancel token.
#[tokio::test]
async fn cancel() -> anyhow::Result<()> {
    let relay = spawn_relay().await?;
    let (intent, key) = intent(A, B, now() + 60)?;
    relay.post(&intent).await?;

    let wrong = CancelKey(key.token());
    assert!(relay.cancel(intent.id(), &wrong).await.is_err());
    assert!(relay.cancel(B256::ZERO, &key).await.is_err());
    assert_eq!(relay.list(&IntentFilter::default()).await?.len(), 1);

    relay.cancel(intent.id(), &key).await?;
    assert!(relay.list(&IntentFilter::default()).await?.is_empty());
    Ok(())
}

fn intent(offer: Address, want: Address, expiry: u64) -> anyhow::Result<(Intent, CancelKey)> {
    let receiver = Account::from_seed(&[1; 32], Policy::NONE).pub_account();
    let mut rng = StdRng::seed_from_u64(expiry ^ u64::from(offer.0[0]));
    let offer = (AssetId(offer), 100);
    let want = (AssetId(want), 50);
    Ok(Intent::new(
        Address::ZERO,
        offer,
        want,
        &receiver,
        expiry,
        &mut rng,
    )?)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
