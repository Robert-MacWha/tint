package operation

import (
	"testing"

	frbn254 "github.com/consensys/gnark-crypto/ecc/bn254/fr"

	"github.com/Robert-MacWha/tint/packages/crates/tint-multisig-spendability/go/internal/poseidon2"
)

func mustFr(s string) frbn254.Element {
	var e frbn254.Element
	if _, err := e.SetString(s); err != nil {
		panic(err)
	}
	return e
}

func frOf32(b byte) frbn254.Element {
	var word [32]byte
	for i := range word {
		word[i] = b
	}
	var e frbn254.Element
	e.SetBytes(word[:])
	return e
}

// TestBaseCommitmentHashSnapshotVector cross-checks the full commitment-hash
// chain against packages/crates/tint/src/note/commitment.rs's existing insta
// snapshot (via
// packages/crates/tint-multisig-spendability/tests/dump_poseidon2_constants.rs),
// for SpendableCommitment::new(AssetId([1;20]), 100, NullifierKey::default(),
// [2;20], Fr(3), Bytes::default(), [5;32]). address_to_fr([1;20]) and
// address_to_fr([2;20]) are Rust's job now (this package only ever receives
// pre-converted Fr values), so the known decimal results are hardcoded here
// as the "already converted" inputs a real caller would supply.
func TestBaseCommitmentHashSnapshotVector(t *testing.T) {
	assetFr := mustFr("5731378969925109483151705226338364782964441345")             // address_to_fr([1;20])
	spendabilityAddrFr := mustFr("11462757939850218966303410452676729565928882690") // address_to_fr([2;20])

	witness := frbn254.NewElement(3)
	spendabilityHash := poseidon2.Compress2([2]frbn254.Element{spendabilityAddrFr, witness})
	if got := spendabilityHash.String(); got != "2716180595393461892644672706880765854318048713130920396796110293262763102199" {
		t.Fatalf("spendability_hash([2;20], 3) = %s", got)
	}

	nullifierPubKey := poseidon2.Compress2([2]frbn254.Element{{}, {}}) // NullifierKey::default() = 0
	if got := nullifierPubKey.String(); got != "15621590199821056450610068202457788725601603091791048810523422053872049975191" {
		t.Fatalf("nullifier_pub_key.0 = %s", got)
	}

	base := BaseCommitment{
		AssetFr:          assetFr,
		AmountFr:         frbn254.NewElement(100),
		SpendabilityHash: spendabilityHash,
		RandomFr:         frOf32(5),
		NullifierPubKey:  nullifierPubKey,
	}

	partialHash := base.PartialHash()
	if got := partialHash.String(); got != "4682638111924587591427498308712822513952806315113979350414371467408771787593" {
		t.Fatalf("BaseCommitment.PartialHash() = %s", got)
	}
	baseHash := base.Hash()
	if got := baseHash.String(); got != "151122391010099193331386929876946401472211150702802670594863584012381564898" {
		t.Fatalf("BaseCommitment.Hash() = %s", got)
	}

	// SpendableCommitment.Hash() delegates to Inner, matching
	// spendable_commitment.hash() == base_commitment.hash() in the Rust test.
	spendable := SpendableCommitment{Inner: base}
	spendableInnerHash := spendable.Inner.Hash()
	if spendableInnerHash.String() != baseHash.String() {
		t.Fatalf("SpendableCommitment.Inner.Hash() != base.Hash()")
	}
}

// TestOperationHashZeroAmountSkipsHash is a self-consistency check: a
// default (all-zero-amount) Operation's hash is exactly the T=2 compress of
// (0,0) folded NInputs+NOutputs+NWithdrawals times from a zero seed.
func TestOperationHashZeroAmountSkipsHash(t *testing.T) {
	var op Operation
	got := op.Hash()

	want := frbn254.Element{}
	for range NInputs + NOutputs + NWithdrawals {
		want = poseidon2.Compress2([2]frbn254.Element{want, {}})
	}

	if got.String() != want.String() {
		t.Fatalf("Operation{}.Hash() = %s, want %s", got.String(), want.String())
	}
}

// TestOperationHashMultipleInputs is a regression/shape check (not a fixed
// value): populating two input slots with nonzero amounts should change the
// hash relative to an all-zero operation, mirroring
// tint-password-spendability's multiple_inputs test shape.
func TestOperationHashMultipleInputs(t *testing.T) {
	var zero Operation
	zeroHash := zero.Hash()

	var op Operation
	op.Inputs[0] = SpendableCommitment{
		Inner: BaseCommitment{
			AssetFr:  mustFr("5731378969925109483151705226338364782964441345"),
			AmountFr: frbn254.NewElement(100),
			RandomFr: frOf32(5),
		},
	}
	op.Inputs[1] = SpendableCommitment{
		Inner: BaseCommitment{
			AssetFr:  mustFr("5731378969925109483151705226338364782964441345"),
			AmountFr: frbn254.NewElement(100),
			RandomFr: frOf32(5),
		},
	}

	opHash := op.Hash()
	if opHash.String() == zeroHash.String() {
		t.Fatalf("Operation.Hash() did not change when inputs were populated")
	}
}
