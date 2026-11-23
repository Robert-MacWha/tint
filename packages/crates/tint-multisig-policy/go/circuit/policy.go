package circuit

import (
	"github.com/consensys/gnark/frontend"

	"github.com/Robert-MacWha/tint/packages/crates/tint-multisig-policy/go/internal/operation"
	"github.com/Robert-MacWha/tint/packages/crates/tint-multisig-policy/go/internal/poseidon2"
)

// Checks shared by the policy circuits in this package.

// assertSigningHash derives the signing hash from the witnessed operation and
// context hash and binds it to the public signingHash.
func assertSigningHash(api frontend.API, op *operation.OperationVar, contextHash, signingHash frontend.Variable) {
	operationHash := op.HashGadget(api)
	derived := poseidon2.CompressGadget2(api, [2]frontend.Variable{operationHash, contextHash})
	api.AssertIsEqual(derived, signingHash)
}

// assertPolicyHashes binds each input's `PolicyHash` to its witnessed
// `PolicyAddress` and `PolicyWitness`, mirroring `OwnedNoteVar::policy_hash`.
func assertPolicyHashes(api frontend.API, op *operation.OperationVar) {
	for i := range op.Inputs {
		input := &op.Inputs[i]
		api.AssertIsEqual(input.Inner.PolicyHash, input.PolicyHashGadget(api))
	}
}

// assertPolicyWitness asserts every input whose `policy_address` matches the
// public policyAddress has `policy_witness` equal to expectedWitness.
func assertPolicyWitness(api frontend.API, op *operation.OperationVar, policyAddress, expectedWitness frontend.Variable) {
	for i := range op.Inputs {
		addressesEq := api.IsZero(api.Sub(op.Inputs[i].PolicyAddress, policyAddress))
		diff := api.Sub(op.Inputs[i].PolicyWitness, expectedWitness)
		api.AssertIsEqual(api.Select(addressesEq, diff, 0), 0)
	}
}
