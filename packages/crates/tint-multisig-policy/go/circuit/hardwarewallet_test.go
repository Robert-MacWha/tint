package circuit

import (
	"bytes"
	"crypto/rand"
	"math/big"
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	frbn254 "github.com/consensys/gnark-crypto/ecc/bn254/fr"
	k1ecdsa "github.com/consensys/gnark-crypto/ecc/secp256k1/ecdsa"
	"github.com/consensys/gnark/std/math/emulated"
	stdecdsa "github.com/consensys/gnark/std/signature/ecdsa"
	"github.com/consensys/gnark/test"

	"github.com/Robert-MacWha/tint/packages/crates/tint-multisig-policy/go/internal/poseidon2"
)

type hardwareWalletCircuit = HardwareWalletPolicyCircuit[emulated.Secp256k1Fp, emulated.Secp256k1Fr]

func TestHardwareWalletValid(t *testing.T) {
	assignment := buildHardwareWalletAssignment(t, genKeypair(t), 1, randomDigest(t))
	checkHardwareWallet(t, assignment, true)
}

func TestHardwareWalletMultipleInputsValid(t *testing.T) {
	assignment := buildHardwareWalletAssignment(t, genKeypair(t), 3, randomDigest(t))
	checkHardwareWallet(t, assignment, true)
}

// A digest above secp256k1's scalar modulus is read mod n, as ECDSA does.
func TestHardwareWalletDigestAboveModulusValid(t *testing.T) {
	digest := [32]byte(bytes.Repeat([]byte{0xff}, 32))
	assignment := buildHardwareWalletAssignment(t, genKeypair(t), 1, digest)
	checkHardwareWallet(t, assignment, true)
}

func TestHardwareWalletWrongDigestFails(t *testing.T) {
	assignment := buildHardwareWalletAssignment(t, genKeypair(t), 1, randomDigest(t))
	hi, lo := DigestLimbs(randomDigest(t))
	assignment.DigestHi, assignment.DigestLo = hi, lo
	checkHardwareWallet(t, assignment, false)
}

func TestHardwareWalletOversizedDigestLimbFails(t *testing.T) {
	digest := randomDigest(t)
	assignment := buildHardwareWalletAssignment(t, genKeypair(t), 1, digest)
	// lo + 2^128 with hi - 1 encodes the same 256-bit value with a 129-bit lo.
	hi, lo := DigestLimbs(digest)
	assignment.DigestHi = new(big.Int).Sub(hi, big.NewInt(1))
	assignment.DigestLo = new(big.Int).Add(lo, twoTo128)
	checkHardwareWallet(t, assignment, false)
}

func TestHardwareWalletOtherSignerFails(t *testing.T) {
	digest := randomDigest(t)
	assignment := buildHardwareWalletAssignment(t, genKeypair(t), 1, digest)
	assignment.Signature = signDigest(t, genKeypair(t).priv, digest)
	checkHardwareWallet(t, assignment, false)
}

func TestHardwareWalletInvalidSigningHashFails(t *testing.T) {
	assignment := buildHardwareWalletAssignment(t, genKeypair(t), 1, randomDigest(t))
	assignment.SigningHash = "1"
	checkHardwareWallet(t, assignment, false)
}

// An attacker's key doesn't open a note committed to another signer.
func TestHardwareWalletAttackerKeyFails(t *testing.T) {
	digest := randomDigest(t)
	assignment := buildHardwareWalletAssignment(t, genKeypair(t), 1, digest)
	attacker := genKeypair(t)
	assignment.PubKey = pubKeyVar(attacker.xy)
	assignment.Signature = signDigest(t, attacker.priv, digest)
	checkHardwareWallet(t, assignment, false)
}

func TestHardwareWalletOffCurvePubKeyFails(t *testing.T) {
	key := genKeypair(t)
	key.xy.Y = new(big.Int).Add(key.xy.Y, big.NewInt(1))
	assignment := buildHardwareWalletAssignment(t, key, 1, randomDigest(t))
	checkHardwareWallet(t, assignment, false)
}

func checkHardwareWallet(t *testing.T, assignment *hardwareWalletCircuit, valid bool) {
	t.Helper()
	opt := test.WithInvalidAssignment(assignment)
	if valid {
		opt = test.WithValidAssignment(assignment)
	}
	test.NewAssert(t).CheckCircuit(&hardwareWalletCircuit{}, opt, test.WithCurves(ecc.BN254))
}

// buildHardwareWalletAssignment builds a witness with nInputs operation
// inputs bound to key, and key's signature over digest.
func buildHardwareWalletAssignment(t testing.TB, key keypair, nInputs int, digest [32]byte) *hardwareWalletCircuit {
	t.Helper()
	witness := PubKeyHash([]PubKeyXY{key.xy})
	policyAddress := frbn254.NewElement(9)
	op := buildOperation(nInputs, policyAddress, witness)
	contextHash := frbn254.NewElement(7)
	signingHash := poseidon2.Compress2([2]frbn254.Element{op.Hash(), contextHash})
	hi, lo := DigestLimbs(digest)

	return &hardwareWalletCircuit{
		PolicyAddress: policyAddress.String(),
		SigningHash:   signingHash.String(),
		DigestHi:      hi,
		DigestLo:      lo,
		Operation:     op.ToVar(),
		ContextHash:   contextHash.String(),
		PubKey:        pubKeyVar(key.xy),
		Signature:     signDigest(t, key.priv, digest),
	}
}

func randomDigest(t testing.TB) [32]byte {
	var digest [32]byte
	if _, err := rand.Read(digest[:]); err != nil {
		t.Fatal(err)
	}
	return digest
}

func signDigest(t testing.TB, priv *k1ecdsa.PrivateKey, digest [32]byte) stdecdsa.Signature[emulated.Secp256k1Fr] {
	sigBin, err := priv.Sign(digest[:], nil)
	if err != nil {
		t.Fatal(err)
	}
	var sig k1ecdsa.Signature
	if _, err := sig.SetBytes(sigBin); err != nil {
		t.Fatal(err)
	}
	return stdecdsa.Signature[emulated.Secp256k1Fr]{
		R: emulated.ValueOf[emulated.Secp256k1Fr](new(big.Int).SetBytes(sig.R[:32])),
		S: emulated.ValueOf[emulated.Secp256k1Fr](new(big.Int).SetBytes(sig.S[:32])),
	}
}
