package main

/*
#include "tint.h"
#include <stdlib.h>
*/
import "C"

import (
	"bytes"
	"fmt"
	"io"
	"unsafe"

	"golang.org/x/crypto/sha3"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend"
	"github.com/consensys/gnark/backend/groth16"
	bn254groth16 "github.com/consensys/gnark/backend/groth16/bn254"
	bn254cs "github.com/consensys/gnark/constraint/bn254"
	"github.com/consensys/gnark/frontend"
	gnarkio "github.com/consensys/gnark/io"
)

// TintProve builds the witness for input, computes a Groth16 proof, verifies
// it locally, and converts it into the byte layout expected by the generated
// Solidity verifier.
//
// Returns 0 on success, nonzero on error. On success, outProof is populated.
//
//export TintProve
func TintProve(
	ccsPtr *C.uint8_t, ccsLen C.size_t,
	pkPtr *C.uint8_t, pkLen C.size_t,
	vkPtr *C.uint8_t, vkLen C.size_t,
	input *C.TintProveInput,
	outProof **C.uint8_t, outProofLen *C.size_t,
) C.uint8_t {
	// Load inputs from C
	var ccs bn254cs.R1CS
	if err := fromC(&ccs, ccsPtr, ccsLen); err != nil {
		return 1
	}

	pk := groth16.NewProvingKey(ecc.BN254)
	if err := fromCDump(pk, pkPtr, pkLen); err != nil {
		return 2
	}

	vk := groth16.NewVerifyingKey(ecc.BN254)
	if err := fromC(vk, vkPtr, vkLen); err != nil {
		return 3
	}

	// Build witness
	assignment := fromCProveInput(input)
	fullWitness, err := frontend.NewWitness(&assignment, ecc.BN254.ScalarField())
	if err != nil {
		return 4
	}
	publicWitness, err := fullWitness.Public()
	if err != nil {
		return 5
	}

	// Prove, then verify locally as a sanity check.
	proof, err := groth16.Prove(&ccs, pk, fullWitness, proverHashToField())
	if err != nil {
		return 6
	}
	if err := groth16.Verify(proof, vk, publicWitness, verifierHashToField()); err != nil {
		return 7
	}

	bn254Proof, ok := proof.(*bn254groth16.Proof)
	if !ok {
		return 8
	}
	solidityProof := bn254Proof.MarshalSolidity()

	*outProof = (*C.uint8_t)(C.CBytes(solidityProof))
	*outProofLen = C.size_t(len(solidityProof))
	return 0
}

// Binds Pedersen commitments using plain legacy Keccak256, matching
// ExportSolidity's default.
func proverHashToField() backend.ProverOption {
	return backend.WithProverHashToFieldFunction(sha3.NewLegacyKeccak256())
}

func verifierHashToField() backend.VerifierOption {
	return backend.WithVerifierHashToFieldFunction(sha3.NewLegacyKeccak256())
}

func fromC(w io.ReaderFrom, ptr *C.uint8_t, length C.size_t) error {
	if ptr == nil || length == 0 {
		return nil
	}
	buf := C.GoBytes(unsafe.Pointer(ptr), C.int(length))
	_, err := w.ReadFrom(bytes.NewReader(buf))
	return err
}

func fromCDump(w gnarkio.BinaryDumper, ptr *C.uint8_t, length C.size_t) error {
	fmt.Println("WARNING: proving key loaded via ReadDump (unchecked, no subgroup validation). Only use for testing and development.")

	if ptr == nil || length == 0 {
		return nil
	}
	buf := C.GoBytes(unsafe.Pointer(ptr), C.int(length))
	return w.ReadDump(bytes.NewReader(buf))
}
