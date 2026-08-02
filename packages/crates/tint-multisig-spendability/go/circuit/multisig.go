package circuit

import (
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/std/algebra/algopts"
	"github.com/consensys/gnark/std/algebra/emulated/sw_emulated"
	stdbits "github.com/consensys/gnark/std/math/bits"
	"github.com/consensys/gnark/std/math/emulated"
	stdecdsa "github.com/consensys/gnark/std/signature/ecdsa"

	"github.com/Robert-MacWha/tint/packages/crates/tint-multisig-spendability/go/internal/operation"
)

// MultisigSpendability proves that an operation is spendable under an
// M-of-N secp256k1 multisig rule:
//
// 1. NSigners public keys are committed to via PubKeyHash (bound to the note's
// spendability_witness)
//
// 2. At least `Threshold` of them must have validly signed `operation_hash`.
type MultisigSpendability[Base, Scalar emulated.FieldParams] struct {
	SpendabilityAddress frontend.Variable `gnark:",public"`
	OperationHash       frontend.Variable `gnark:",public"`

	Operation  operation.OperationVar
	PubKeys    [NSigners]stdecdsa.PublicKey[Base, Scalar]
	Signatures [NSigners]stdecdsa.Signature[Scalar]
}

func (c *MultisigSpendability[Base, Scalar]) Define(api frontend.API) error {
	if err := c.verifyOperationHash(api); err != nil {
		return err
	}
	if err := c.verifySpendabilityWitness(api); err != nil {
		return err
	}
	return c.verifyThresholdSignatures(api)
}

// verifyOperationHash derives the operation's hash from the
// witnessed `Operation and binds it to the public `operation_hash`.
func (c *MultisigSpendability[Base, Scalar]) verifyOperationHash(api frontend.API) error {
	api.AssertIsEqual(c.Operation.HashGadget(api), c.OperationHash)
	return nil
}

// verifySpendabilityWitness asserts for every operation input whose
// `spendability_address` matches the public `spendability_address`, the note's
// `spendability_witness` must equal the hash of this rule's committed pubkey
// set.
func (c *MultisigSpendability[Base, Scalar]) verifySpendabilityWitness(api frontend.API) error {
	baseField, err := emulated.NewField[Base](api)
	if err != nil {
		return err
	}
	expectedWitness := PubKeyHashGadget(api, baseField, c.PubKeys[:])

	for i := range c.Operation.Inputs {
		addressesEq := api.IsZero(api.Sub(c.Operation.Inputs[i].SpendabilityAddress, c.SpendabilityAddress))
		diff := api.Sub(c.Operation.Inputs[i].SpendabilityWitness, expectedWitness)
		api.AssertIsEqual(api.Select(addressesEq, diff, 0), 0)
	}
	return nil
}

// verifyThresholdSignatures checks each signer's signature over
// operation_hash and asserts at least `Threshold` of them are valid.
func (c *MultisigSpendability[Base, Scalar]) verifyThresholdSignatures(api frontend.API) error {
	msg, err := c.operationHashAsScalar(api)
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

// operationHashAsScalar carries OperationHash across into an emulated
// scalar-field element.
func (c *MultisigSpendability[Base, Scalar]) operationHashAsScalar(api frontend.API) (*emulated.Element[Scalar], error) {
	scalarField, err := emulated.NewField[Scalar](api)
	if err != nil {
		return nil, err
	}
	hashBits := stdbits.ToBinary(api, c.OperationHash)
	return scalarField.FromBits(hashBits...), nil
}
