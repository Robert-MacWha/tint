//! Cross-checks that the signing hash policies authorize, computed locally,
//! matches what `ProofLib.signingHash` computes on-chain for the same
//! operation.

use alloy_node_bindings::Anvil;
use alloy_primitives::{Address, B256, Bytes};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_macro::sol;
use alloy_sol_types::SolValue;
use ark_std::rand::{SeedableRng, rngs::StdRng};
use tint::{
    account::Account,
    fr::u256_to_fr,
    note::{asset::AssetId, owned::OwnedNote},
    operation::OperationBuilder,
    policy::Policy,
    skew_mmr::SkewMmr,
};

sol!(
    #[sol(rpc)]
    Poseidon2T2,
    "../../contracts/out/Poseidon2T2.sol/Poseidon2T2.json"
);

sol!(
    #[sol(rpc)]
    SigningHash,
    "../../contracts/out/SigningHash.sol/SigningHash.json"
);

#[tokio::test]
#[ignore = "run with `cargo test --release -- --ignored`"]
async fn signing_hash_match_onchain() -> Result<(), Box<dyn std::error::Error>> {
    let anvil = Anvil::new().spawn();
    let signer = PrivateKeySigner::from_slice(&anvil.first_key().to_bytes())?;
    let provider = ProviderBuilder::new()
        .wallet(signer)
        .connect_http(anvil.endpoint().parse()?)
        .erased();

    let poseidon = Poseidon2T2::deploy(provider.clone()).await?;
    let harness = SigningHash::deploy(provider.clone(), *poseidon.address()).await?;

    // Spend a note from a local MMR, with every context field set.
    let mut rng = StdRng::seed_from_u64(1);
    let account = Account::from_seed(&[11u8; 32], Policy::NONE);
    let asset = AssetId::default();
    let note = account.pub_account().commitment(asset, 100, B256::ZERO);
    let note = OwnedNote::new(note, &account)?;
    let mut mmr = SkewMmr::new();
    mmr.append(note.commitment());

    let op = OperationBuilder::new()
        .spend(note)
        .output(account.pub_account(), asset, 60)
        .withdraw(Address::repeat_byte(2), asset, 40)
        .caller(Address::repeat_byte(3))
        .binding(B256::repeat_byte(4))
        .build(&mut rng)?;
    let local = op.operation().signing_hash();

    //? The signing hash doesn't depend on the proof, so simulating computes it without proving.
    let simulated = op.authorize(vec![Bytes::new()])?.simulate_with_mmr(&mmr)?;

    //? The harness's `sol!` binding has its own `Operation` type, so convert
    //? tint's through its ABI encoding.
    let operation = IPrivacyPool::Operation::abi_decode(&simulated.operation().abi_encode())?;
    let onchain = harness.signingHash(operation).call().await?;

    assert_eq!(
        local,
        u256_to_fr(onchain),
        "signing hash does not match on-chain value"
    );

    Ok(())
}
