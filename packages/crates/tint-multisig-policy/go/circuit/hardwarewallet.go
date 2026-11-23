package circuit

import (
	"math/big"

	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/std/algebra/emulated/sw_emulated"
	stdbits "github.com/consensys/gnark/std/math/bits"
	"github.com/consensys/gnark/std/math/emulated"
	stdecdsa "github.com/consensys/gnark/std/signature/ecdsa"

	"github.com/Robert-MacWha/tint/packages/crates/tint-multisig-policy/go/internal/operation"
)

// HardwareWalletPolicyCircuit proves that an operation is spendable under a
// single-signer secp256k1 rule, as signed by a hardware wallet:
//
// 1. The signer's public key is committed to via PubKeyHash (bound to the
// note's policy_witness).
//
// 2. The signer validly signed `Digest`, the EIP-712 digest of
// `TintOperation(bytes32 signingHash)`. The policy contract derives `Digest`
// from the same `signing_hash` it passes in, so the circuit only checks the
// signature over it and binds `signing_hash` to the witnessed `Operation`.
//
// `Digest` is split into 128-bit halves since it doesn't fit the BN254
// scalar field.
type HardwareWalletPolicyCircuit[Base, Scalar emulated.FieldParams] struct {
	PolicyAddress frontend.Variable `gnark:",public"`
	SigningHash   frontend.Variable `gnark:",public"`
	DigestHi      frontend.Variable `gnark:",public"`
	DigestLo      frontend.Variable `gnark:",public"`

	Operation   operation.OperationVar
	ContextHash frontend.Variable
	PubKey      stdecdsa.PublicKey[Base, Scalar]
	Signature   stdecdsa.Signature[Scalar]
}

func (c *HardwareWalletPolicyCircuit[Base, Scalar]) Define(api frontend.API) error {
	assertSigningHash(api, &c.Operation, c.ContextHash, c.SigningHash)
	assertPolicyHashes(api, &c.Operation)

	baseField, err := emulated.NewField[Base](api)
	if err != nil {
		return err
	}
	pubKeys := []stdecdsa.PublicKey[Base, Scalar]{c.PubKey}
	assertPolicyWitness(api, &c.Operation, c.PolicyAddress, PubKeyHashGadget(api, baseField, pubKeys))

	return c.verifySignature(api)
}

// DigestLimbs splits a 32-byte big-endian digest into its [hi, lo] 128-bit
// halves, matching the circuit's `DigestHi`/`DigestLo`.
func DigestLimbs(digest [32]byte) (hi, lo *big.Int) {
	return split128(new(big.Int).SetBytes(digest[:]))
}

// verifySignature asserts the signer's signature over the digest is valid.
func (c *HardwareWalletPolicyCircuit[Base, Scalar]) verifySignature(api frontend.API) error {
	msg, err := c.digestAsScalar(api)
	if err != nil {
		return err
	}

	curveParams := sw_emulated.GetCurveParams[Base]()
	curve, err := sw_emulated.New[Base, Scalar](api, curveParams)
	if err != nil {
		return err
	}
	curve.AssertIsOnCurve((*sw_emulated.AffinePoint[Base])(&c.PubKey))
	c.PubKey.Verify(api, curveParams, msg, &c.Signature)
	return nil
}

// digestAsScalar reassembles the digest's halves into an emulated
// scalar-field element, as ECDSA reads a 256-bit hash.
func (c *HardwareWalletPolicyCircuit[Base, Scalar]) digestAsScalar(api frontend.API) (*emulated.Element[Scalar], error) {
	scalarField, err := emulated.NewField[Scalar](api)
	if err != nil {
		return nil, err
	}
	loBits := stdbits.ToBinary(api, c.DigestLo, stdbits.WithNbDigits(128))
	hiBits := stdbits.ToBinary(api, c.DigestHi, stdbits.WithNbDigits(128))
	return scalarField.FromBits(append(loBits, hiBits...)...), nil
}
