package circuit

import (
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/std/algebra/algopts"
	"github.com/consensys/gnark/std/algebra/emulated/sw_emulated"
	stdbits "github.com/consensys/gnark/std/math/bits"
	"github.com/consensys/gnark/std/math/emulated"
	stdecdsa "github.com/consensys/gnark/std/signature/ecdsa"

	"github.com/Robert-MacWha/tint/packages/crates/tint-multisig-policy/go/internal/operation"
	"github.com/Robert-MacWha/tint/packages/crates/tint-multisig-policy/go/internal/poseidon2"
)

// MultisigPolicyCircuit proves that an operation is spendable under an
// M-of-N secp256k1 multisig rule:
//
// 1. NSigners public keys are committed to via PubKeyHash (bound to the note's
// policy_witness)
//
// 2. At least `Threshold` of them must have validly signed `signing_hash`,
// which commits to the witnessed `Operation` and `ContextHash`.
type MultisigPolicyCircuit[Base, Scalar emulated.FieldParams] struct {
	PolicyAddress frontend.Variable `gnark:",public"`
	SigningHash         frontend.Variable `gnark:",public"`

	Operation   operation.OperationVar
	ContextHash frontend.Variable
	PubKeys     [NSigners]stdecdsa.PublicKey[Base, Scalar]
	Signatures  [NSigners]stdecdsa.Signature[Scalar]
}

func (c *MultisigPolicyCircuit[Base, Scalar]) Define(api frontend.API) error {
	if err := c.verifySigningHash(api); err != nil {
		return err
	}
	if err := c.verifyPolicyWitness(api); err != nil {
		return err
	}
	return c.verifyThresholdSignatures(api)
}

// verifySigningHash derives the signing hash from the witnessed `Operation`
// and `ContextHash` and binds it to the public `signing_hash`.
func (c *MultisigPolicyCircuit[Base, Scalar]) verifySigningHash(api frontend.API) error {
	operationHash := c.Operation.HashGadget(api)
	signingHash := poseidon2.CompressGadget2(api, [2]frontend.Variable{operationHash, c.ContextHash})
	api.AssertIsEqual(signingHash, c.SigningHash)
	return nil
}

// verifyPolicyWitness asserts for every operation input whose
// `policy_address` matches the public `policy_address`, the note's
// `policy_witness` must equal the hash of this rule's committed pubkey
// set.
func (c *MultisigPolicyCircuit[Base, Scalar]) verifyPolicyWitness(api frontend.API) error {
	baseField, err := emulated.NewField[Base](api)
	if err != nil {
		return err
	}
	expectedWitness := PubKeyHashGadget(api, baseField, c.PubKeys[:])

	for i := range c.Operation.Inputs {
		addressesEq := api.IsZero(api.Sub(c.Operation.Inputs[i].PolicyAddress, c.PolicyAddress))
		diff := api.Sub(c.Operation.Inputs[i].PolicyWitness, expectedWitness)
		api.AssertIsEqual(api.Select(addressesEq, diff, 0), 0)
	}
	return nil
}

// verifyThresholdSignatures checks each signer's signature over
// signing_hash and asserts at least `Threshold` of them are valid.
func (c *MultisigPolicyCircuit[Base, Scalar]) verifyThresholdSignatures(api frontend.API) error {
	msg, err := c.signingHashAsScalar(api)
	if err != nil {
		return err
	}

	curveParams := sw_emulated.GetCurveParams[Base]()
	var validCount frontend.Variable = 0
	for i := range c.PubKeys {
		valid := c.PubKeys[i].IsValid(api, curveParams, msg, &c.Signatures[i], algopts.WithIncompleteArithmetic())
		validCount = api.Add(validCount, valid)
	}
	api.AssertIsLessOrEqual(Threshold, validCount)
	return nil
}

// signingHashAsScalar carries SigningHash across into an emulated
// scalar-field element.
func (c *MultisigPolicyCircuit[Base, Scalar]) signingHashAsScalar(api frontend.API) (*emulated.Element[Scalar], error) {
	scalarField, err := emulated.NewField[Scalar](api)
	if err != nil {
		return nil, err
	}
	hashBits := stdbits.ToBinary(api, c.SigningHash)
	return scalarField.FromBits(hashBits...), nil
}
