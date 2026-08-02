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

type keypair struct {
	priv *k1ecdsa.PrivateKey
	xy   PubKeyXY
}

func genKeypair(t testing.TB) keypair {
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

func signOperationHash(t testing.TB, priv *k1ecdsa.PrivateKey, opHash frbn254.Element) stdecdsa.Signature[emulated.Secp256k1Fr] {
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

// buildOperation returns a native Operation with nInputs slots populated.
func buildOperation(nInputs int, spendabilityAddress frbn254.Element, witness frbn254.Element) operation.Operation {
	var op operation.Operation
	for i := range nInputs {
		op.Inputs[i] = operation.SpendableCommitment{
			Inner: operation.BaseCommitment{
				AssetFr:  frbn254.NewElement(1),
				AmountFr: frbn254.NewElement(100),
				RandomFr: frbn254.NewElement(42),
			},
			SpendabilityAddress: spendabilityAddress,
			SpendabilityWitness: witness,
		}
	}
	return op
}

// buildAssignment builds a MultisigSpendability witness with nInputs
// populated operation-input slots and nValidSigs real signatures over the
// resulting operation hash.
func buildAssignment(t testing.TB, nInputs, nValidSigs int) (*MultisigSpendability[emulated.Secp256k1Fp, emulated.Secp256k1Fr], [NSigners]keypair, frbn254.Element) {
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
			// Unused slots are zeroed
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

func TestMultipleInputsValid(t *testing.T) {
	assignment, _, _ := buildAssignment(t, 3, Threshold)

	assert := test.NewAssert(t)
	assert.CheckCircuit(&MultisigSpendability[emulated.Secp256k1Fp, emulated.Secp256k1Fr]{}, test.WithValidAssignment(assignment), test.WithCurves(ecc.BN254))
}

func TestInvalidSignatureCountFails(t *testing.T) {
	assignment, _, _ := buildAssignment(t, 1, Threshold-1)

	assert := test.NewAssert(t)
	assert.CheckCircuit(&MultisigSpendability[emulated.Secp256k1Fp, emulated.Secp256k1Fr]{}, test.WithInvalidAssignment(assignment), test.WithCurves(ecc.BN254))
}

func TestInvalidOperationHashFails(t *testing.T) {
	assignment, _, opHash := buildAssignment(t, 1, Threshold)

	tampered := new(big.Int).Add(opHash.BigInt(new(big.Int)), big.NewInt(1))
	assignment.OperationHash = tampered.String()

	assert := test.NewAssert(t)
	assert.CheckCircuit(&MultisigSpendability[emulated.Secp256k1Fp, emulated.Secp256k1Fr]{}, test.WithInvalidAssignment(assignment), test.WithCurves(ecc.BN254))
}

func TestWrongPubKeyHashFails(t *testing.T) {
	assignment, _, _ := buildAssignment(t, 1, Threshold)

	otherKeys := [NSigners]PubKeyXY{genKeypair(t).xy, genKeypair(t).xy, genKeypair(t).xy}
	wrongWitness := PubKeyHash(otherKeys[:])
	assignment.Operation.Inputs[0].SpendabilityWitness = wrongWitness.String()

	assert := test.NewAssert(t)
	assert.CheckCircuit(&MultisigSpendability[emulated.Secp256k1Fp, emulated.Secp256k1Fr]{}, test.WithInvalidAssignment(assignment), test.WithCurves(ecc.BN254))
}
