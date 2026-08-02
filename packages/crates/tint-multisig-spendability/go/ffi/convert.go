package main

/*
#include "tint.h"
*/
import "C"

import (
	"math/big"

	frbn254 "github.com/consensys/gnark-crypto/ecc/bn254/fr"
	"github.com/consensys/gnark/std/math/emulated"
	stdecdsa "github.com/consensys/gnark/std/signature/ecdsa"

	"github.com/Robert-MacWha/tint/packages/crates/tint-multisig-spendability/go/circuit"
	"github.com/Robert-MacWha/tint/packages/crates/tint-multisig-spendability/go/internal/operation"
)

func fromCProveInput(input *C.TintProveInput) circuit.MultisigSpendability[emulated.Secp256k1Fp, emulated.Secp256k1Fr] {
	operation := fromCOperation(&input.operation)
	var pubKeys [circuit.NSigners]stdecdsa.PublicKey[emulated.Secp256k1Fp, emulated.Secp256k1Fr]
	var signatures [circuit.NSigners]stdecdsa.Signature[emulated.Secp256k1Fr]

	for i := range pubKeys {
		key := fromCPubKeyXY(input.pub_keys[i])
		sig := fromCSignatureRS(input.signatures[i])
		pubKeys[i] = stdecdsa.PublicKey[emulated.Secp256k1Fp, emulated.Secp256k1Fr]{
			X: emulated.ValueOf[emulated.Secp256k1Fp](key.X),
			Y: emulated.ValueOf[emulated.Secp256k1Fp](key.Y),
		}
		signatures[i] = stdecdsa.Signature[emulated.Secp256k1Fr]{
			R: emulated.ValueOf[emulated.Secp256k1Fr](sig.R),
			S: emulated.ValueOf[emulated.Secp256k1Fr](sig.S),
		}
	}

	spendabilityAddress := fromCFr(input.spendability_address)
	operationHash := operation.Hash()

	return circuit.MultisigSpendability[emulated.Secp256k1Fp, emulated.Secp256k1Fr]{
		SpendabilityAddress: spendabilityAddress.String(),
		OperationHash:       operationHash.String(),
		Operation:           operation.ToVar(),
		PubKeys:             pubKeys,
		Signatures:          signatures,
	}
}

func fromCOperation(op *C.TintOperation) operation.Operation {
	var result operation.Operation
	for i := range result.Inputs {
		result.Inputs[i] = fromCSpendableCommitment(op.inputs[i])
	}
	for i := range result.OutputCommitments {
		result.OutputCommitments[i] = fromCBaseCommitment(op.output_commitments[i])
	}
	for i := range result.OutputWithdrawals {
		result.OutputWithdrawals[i] = fromCWithdrawal(op.output_withdrawals[i])
	}
	return result
}

func fromCSpendableCommitment(c C.TintSpendableCommitment) operation.SpendableCommitment {
	return operation.SpendableCommitment{
		Inner:               fromCBaseCommitment(c.inner),
		SpendabilityAddress: fromCFr(c.spendability_address),
		SpendabilityWitness: fromCFr(c.spendability_witness),
	}
}

func fromCBaseCommitment(c C.TintBaseCommitment) operation.BaseCommitment {
	return operation.BaseCommitment{
		AssetFr:          fromCFr(c.asset_fr),
		AmountFr:         fromCFr(c.amount_fr),
		SpendabilityHash: fromCFr(c.spendability_hash),
		RandomFr:         fromCFr(c.random_fr),
		NullifierPubKey:  fromCFr(c.nullifier_pub_key),
	}
}

func fromCWithdrawal(c C.TintWithdrawal) operation.Withdrawal {
	return operation.Withdrawal{
		AssetFr:  fromCFr(c.asset_fr),
		AmountFr: fromCFr(c.amount_fr),
	}
}

func fromCPubKeyXY(p C.TintPubKeyXY) circuit.PubKeyXY {
	return circuit.PubKeyXY{X: fromCBigInt(p.x), Y: fromCBigInt(p.y)}
}

func fromCSignatureRS(s C.TintSignatureRS) circuit.SignatureRS {
	return circuit.SignatureRS{R: fromCBigInt(s.r), S: fromCBigInt(s.s)}
}

func fromCBytes32(b C.TintBytes32) [32]byte {
	var out [32]byte
	for i := range out {
		out[i] = byte(b.data[i])
	}
	return out
}

func toCBytes32(b [32]byte) C.TintBytes32 {
	var out C.TintBytes32
	for i := range b {
		out.data[i] = C.uint8_t(b[i])
	}
	return out
}

func fromCFr(b C.TintBytes32) frbn254.Element {
	bytes := fromCBytes32(b)
	var e frbn254.Element
	e.SetBytes(bytes[:])
	return e
}

func toCFr(e frbn254.Element) C.TintBytes32 {
	return toCBytes32(e.Bytes())
}

func fromCBigInt(b C.TintBytes32) *big.Int {
	bytes := fromCBytes32(b)
	return new(big.Int).SetBytes(bytes[:])
}
