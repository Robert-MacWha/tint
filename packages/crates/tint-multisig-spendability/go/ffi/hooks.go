package main

/*
#include "tint.h"
*/
import "C"

import (
	"unsafe"

	frbn254 "github.com/consensys/gnark-crypto/ecc/bn254/fr"

	"github.com/Robert-MacWha/tint/packages/crates/tint-multisig-spendability/go/circuit"
	"github.com/Robert-MacWha/tint/packages/crates/tint-multisig-spendability/go/internal/poseidon2"
)

//export Poseidon2Compress1
func Poseidon2Compress1(a *C.Bytes32, out *C.Bytes32) {
	result := poseidon2.Compress1(fromCFr(*a))
	*out = toCFr(result)
}

//export Poseidon2Compress2
func Poseidon2Compress2(a, b *C.Bytes32, out *C.Bytes32) {
	result := poseidon2.Compress2([2]frbn254.Element{fromCFr(*a), fromCFr(*b)})
	*out = toCFr(result)
}

//export Poseidon2Compress3
func Poseidon2Compress3(a, b, c *C.Bytes32, out *C.Bytes32) {
	result := poseidon2.Compress3([3]frbn254.Element{fromCFr(*a), fromCFr(*b), fromCFr(*c)})
	*out = toCFr(result)
}

//export Poseidon2Compress8
func Poseidon2Compress8(a, b, c, d, e, f, g, h *C.Bytes32, out *C.Bytes32) {
	inSlice := []*C.Bytes32{a, b, c, d, e, f, g, h}
	var arr [8]frbn254.Element
	for i := range arr {
		arr[i] = fromCFr(*inSlice[i])
	}
	result := poseidon2.Compress8(arr)
	*out = toCFr(result)
}

//export OperationHash
func OperationHash(op *C.TintOperation, out *C.Bytes32) {
	result := fromCOperation(op).Hash()
	*out = toCFr(result)
}

//export PubKeyHash
func PubKeyHash(pubKeys *C.TintPubKeyXY, out *C.Bytes32) {
	inSlice := unsafe.Slice(pubKeys, circuit.NSigners)
	keys := make([]circuit.PubKeyXY, circuit.NSigners)
	for i := range keys {
		keys[i] = fromCPubKeyXY(inSlice[i])
	}
	result := circuit.PubKeyHash(keys)
	*out = toCFr(result)
}
