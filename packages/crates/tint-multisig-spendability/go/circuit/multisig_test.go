package circuit

import (
	"crypto/rand"
	"math/big"
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	frbn254 "github.com/consensys/gnark-crypto/ecc/bn254/fr"
	k1ecdsa "github.com/consensys/gnark-crypto/ecc/secp256k1/ecdsa"
	"github.com/consensys/gnark/std/math/emulated"
	stdecdsa "github.com/consensys/gnark/std/signature/ecdsa"
	"github.com/consensys/gnark/test"

	"github.com/Robert-MacWha/tint/packages/crates/tint-multisig-spendability/go/internal/operation"
)

// frFill mirrors what Rust would hand Go for a 32-byte-word-shaped value
// (e.g. B256 randomness) filled with a single repeated byte, reduced mod
// BN254's order — this package never converts raw addresses/bytes itself.
func frFill(b byte) frbn254.Element {
	var word [32]byte
	for i := range word {
		word[i] = b
	}
	var e frbn254.Element
	e.SetBytes(word[:])
	return e
}

type keypair struct {
	priv *k1ecdsa.PrivateKey
	xy   PubKeyXY
}

func genKeypair(t *testing.T) keypair {
	priv, err := k1ecdsa.GenerateKey(rand.Reader)
	if err != nil {
		t.Fatal(err)
	}
	x, y := new(big.Int), new(big.Int)
	priv.PublicKey.A.X.BigInt(x)
	priv.PublicKey.A.Y.BigInt(y)
	return keypair{priv: priv, xy: PubKeyXY{X: x, Y: y}}
}

func pubKeyVar(xy PubKeyXY) stdecdsa.PublicKey[emulated.Secp256k1Fp, emulated.Secp256k1Fr] {
	return stdecdsa.PublicKey[emulated.Secp256k1Fp, emulated.Secp256k1Fr]{
		X: emulated.ValueOf[emulated.Secp256k1Fp](xy.X),
		Y: emulated.ValueOf[emulated.Secp256k1Fp](xy.Y),
	}
}

// signOperationHash signs the 32-byte big-endian encoding of an operation
// hash directly (hFunc=nil, so k1ecdsa.HashToInt treats the bytes as the
// scalar value itself, unhashed) so the resulting signature verifies against
// exactly the same scalar the circuit derives from OperationHash.
func signOperationHash(t *testing.T, priv *k1ecdsa.PrivateKey, opHash frbn254.Element) stdecdsa.Signature[emulated.Secp256k1Fr] {
	msgBytes := opHash.Bytes()
	sigBin, err := priv.Sign(msgBytes[:], nil)
	if err != nil {
		t.Fatal(err)
	}
	var sig k1ecdsa.Signature
	sig.SetBytes(sigBin)
	r, s := new(big.Int), new(big.Int)
	r.SetBytes(sig.R[:32])
	s.SetBytes(sig.S[:32])
	return stdecdsa.Signature[emulated.Secp256k1Fr]{
		R: emulated.ValueOf[emulated.Secp256k1Fr](r),
		S: emulated.ValueOf[emulated.Secp256k1Fr](s),
	}
}

// buildOperation returns a native Operation with nInputs slots populated
// (identical spendabilityAddress/witness in each), and its hash.
func buildOperation(nInputs int, spendabilityAddress frbn254.Element, witness frbn254.Element) operation.Operation {
	var op operation.Operation
	for i := range nInputs {
		op.Inputs[i] = operation.SpendableCommitment{
			Inner: operation.BaseCommitment{
				AssetFr:  frFill(1),
				AmountFr: frbn254.NewElement(100),
				RandomFr: frFill(byte(10 + i)),
			},
			SpendabilityAddress: spendabilityAddress,
			SpendabilityWitness: witness,
		}
	}
	return op
}

// buildAssignment builds a MultisigSpendability witness with nInputs
// populated operation-input slots (all sharing the same
// spendability_address/witness) and nValidSigs real signatures over the
// resulting operation hash (the rest are zero-signature unused slots, which
// PublicKey.IsValid treats as automatically invalid).
func buildAssignment(t *testing.T, nInputs, nValidSigs int) (*MultisigSpendability[emulated.Secp256k1Fp, emulated.Secp256k1Fr], [NSigners]keypair, frbn254.Element) {
	t.Helper()

	keys := [NSigners]keypair{genKeypair(t), genKeypair(t), genKeypair(t)}
	var pubKeysXY [NSigners]PubKeyXY
	for i, k := range keys {
		pubKeysXY[i] = k.xy
	}
	witness := PubKeyHash(pubKeysXY[:])
	spendabilityAddress := frbn254.NewElement(9)

	op := buildOperation(nInputs, spendabilityAddress, witness)
	opHash := op.Hash()

	var signatures [NSigners]stdecdsa.Signature[emulated.Secp256k1Fr]
	var pubKeys [NSigners]stdecdsa.PublicKey[emulated.Secp256k1Fp, emulated.Secp256k1Fr]
	for i, k := range keys {
		pubKeys[i] = pubKeyVar(k.xy)
		if i < nValidSigs {
			signatures[i] = signOperationHash(t, k.priv, opHash)
		} else {
			// Unused slot: keep a real on-curve pubkey (required for
			// WithIncompleteArithmetic, which doesn't handle the
			// point-at-infinity edge case) but a zero signature.
			signatures[i] = stdecdsa.Signature[emulated.Secp256k1Fr]{
				R: emulated.ValueOf[emulated.Secp256k1Fr](0),
				S: emulated.ValueOf[emulated.Secp256k1Fr](0),
			}
		}
	}

	return &MultisigSpendability[emulated.Secp256k1Fp, emulated.Secp256k1Fr]{
		SpendabilityAddress: spendabilityAddress.String(),
		OperationHash:       opHash.String(),
		Operation:           op.ToVar(),
		PubKeys:             pubKeys,
		Signatures:          signatures,
	}, keys, opHash
}

func TestValidCircuit(t *testing.T) {
	assignment, _, _ := buildAssignment(t, 1, Threshold)

	assert := test.NewAssert(t)
	assert.CheckCircuit(&MultisigSpendability[emulated.Secp256k1Fp, emulated.Secp256k1Fr]{}, test.WithValidAssignment(assignment), test.WithCurves(ecc.BN254))
}

// TestMultipleInputsValid mirrors tint-password-spendability's
// multiple_inputs test shape: several operation-input slots share the same
// spendability_address/witness and the circuit should still be satisfiable.
func TestMultipleInputsValid(t *testing.T) {
	assignment, _, _ := buildAssignment(t, 3, Threshold)

	assert := test.NewAssert(t)
	assert.CheckCircuit(&MultisigSpendability[emulated.Secp256k1Fp, emulated.Secp256k1Fr]{}, test.WithValidAssignment(assignment), test.WithCurves(ecc.BN254))
}

// TestInvalidSignatureCountFails mirrors password-spendability's
// invalid_secret test: fewer than Threshold valid signatures must fail.
func TestInvalidSignatureCountFails(t *testing.T) {
	assignment, _, _ := buildAssignment(t, 1, Threshold-1)

	assert := test.NewAssert(t)
	assert.CheckCircuit(&MultisigSpendability[emulated.Secp256k1Fp, emulated.Secp256k1Fr]{}, test.WithInvalidAssignment(assignment), test.WithCurves(ecc.BN254))
}

// TestInvalidOperationHashFails mirrors password-spendability's
// invalid_operation_hash test.
func TestInvalidOperationHashFails(t *testing.T) {
	assignment, _, opHash := buildAssignment(t, 1, Threshold)

	tampered := new(big.Int).Add(opHash.BigInt(new(big.Int)), big.NewInt(1))
	assignment.OperationHash = tampered.String()

	assert := test.NewAssert(t)
	assert.CheckCircuit(&MultisigSpendability[emulated.Secp256k1Fp, emulated.Secp256k1Fr]{}, test.WithInvalidAssignment(assignment), test.WithCurves(ecc.BN254))
}

// TestWrongPubKeyHashFails is the multisig-specific analogue of
// password-spendability's invalid_secret test: valid signatures from a real
// keyset, but the note's spendability_witness commits to a different keyset.
func TestWrongPubKeyHashFails(t *testing.T) {
	assignment, _, _ := buildAssignment(t, 1, Threshold)

	otherKeys := [NSigners]PubKeyXY{genKeypair(t).xy, genKeypair(t).xy, genKeypair(t).xy}
	wrongWitness := PubKeyHash(otherKeys[:])
	assignment.Operation.Inputs[0].SpendabilityWitness = wrongWitness.String()

	assert := test.NewAssert(t)
	assert.CheckCircuit(&MultisigSpendability[emulated.Secp256k1Fp, emulated.Secp256k1Fr]{}, test.WithInvalidAssignment(assignment), test.WithCurves(ecc.BN254))
}
