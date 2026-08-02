package main

/*
#include "tint.h"
#include <stdlib.h>
*/
import "C"

import (
	"bytes"
	"io"
	"unsafe"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/backend/witness"
	bn254cs "github.com/consensys/gnark/constraint/bn254"
	"github.com/consensys/gnark/frontend"
)

// TintProve builds the witness for input and computes a Groth16 proof. Returns
// 0 on success, nonzero on error. On success, outPublicInputs and outProof are
// populated.
//
//export TintProve
func TintProve(
	ccsPtr *C.uint8_t, ccsLen C.size_t,
	pkPtr *C.uint8_t, pkLen C.size_t,
	input *C.TintProveInput,
	outPublicInputs **C.uint8_t, outPublicInputsLen *C.size_t,
	outProof **C.uint8_t, outProofLen *C.size_t,
) C.uint8_t {
	// Load inputs from C
	var ccs bn254cs.R1CS
	if err := fromC(&ccs, ccsPtr, ccsLen); err != nil {
		return 1
	}

	pk := groth16.NewProvingKey(ecc.BN254)
	if err := fromC(pk, pkPtr, pkLen); err != nil {
		return 2
	}

	// Build witness
	assignment := fromCProveInput(input)
	fullWitness, err := frontend.NewWitness(&assignment, ecc.BN254.ScalarField())
	if err != nil {
		return 3
	}
	publicWitness, err := fullWitness.Public()
	if err != nil {
		return 4
	}

	// Prove
	proof, err := groth16.Prove(&ccs, pk, fullWitness)
	if err != nil {
		return 5
	}

	// Write outputs to C
	publicInputsPtr, publicInputsLen, err := toC(publicWitness)
	if err != nil {
		return 6
	}
	proofPtr, proofLen, err := toC(proof)
	if err != nil {
		//? Free to avoid memory leak, since rust only frees on success
		C.free(unsafe.Pointer(publicInputsPtr))
		return 7
	}

	*outPublicInputs = publicInputsPtr
	*outPublicInputsLen = publicInputsLen
	*outProof = proofPtr
	*outProofLen = proofLen
	return 0
}

// TintVerify checks a proof against vk/publicInputs. Returns 0 if the
// proof is valid, nonzero if invalid or on error.
//
//export TintVerify
func TintVerify(
	proofPtr *C.uint8_t, proofLen C.size_t,
	vkPtr *C.uint8_t, vkLen C.size_t,
	publicInputsPtr *C.uint8_t, publicInputsLen C.size_t,
) C.uint8_t {
	// Load inputs from C
	proof := groth16.NewProof(ecc.BN254)
	if err := fromC(proof, proofPtr, proofLen); err != nil {
		return 1
	}

	vk := groth16.NewVerifyingKey(ecc.BN254)
	if err := fromC(vk, vkPtr, vkLen); err != nil {
		return 2
	}

	publicWitness, err := witness.New(ecc.BN254.ScalarField())
	if err != nil {
		return 3
	}
	if err := fromC(publicWitness, publicInputsPtr, publicInputsLen); err != nil {
		return 4
	}

	// Verify
	if err := groth16.Verify(proof, vk, publicWitness); err != nil {
		return 5
	}
	return 0
}

func fromC(w io.ReaderFrom, ptr *C.uint8_t, length C.size_t) error {
	if ptr == nil || length == 0 {
		return nil
	}
	buf := C.GoBytes(unsafe.Pointer(ptr), C.int(length))
	_, err := w.ReadFrom(bytes.NewReader(buf))
	return err
}

func toC(w io.WriterTo) (*C.uint8_t, C.size_t, error) {
	var buf bytes.Buffer
	if _, err := w.WriteTo(&buf); err != nil {
		return nil, 0, err
	}
	if buf.Len() == 0 {
		return nil, 0, nil
	}
	return (*C.uint8_t)(C.CBytes(buf.Bytes())), C.size_t(buf.Len()), nil
}
