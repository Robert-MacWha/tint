// Package main is the cgo/C ABI entry point for the multisig spendability
// Go library, built via `go build -buildmode=c-archive` into
// libtint_multisig.a + libtint_multisig.h. See tint.h for the shared C
// struct declarations the exported functions (spread across this file,
// convert.go, hooks.go, and prove.go) use.
//
// Every fixed-shape value (Fr elements, secp256k1 coordinates, the
// Operation being proven) crosses the boundary as a plain C struct of
// 32-byte big-endian words — no JSON, no allocation, since the caller owns
// the memory on both sides for the whole call. Only Prove/Verify need a
// dynamic-length convention (raw ptr+len buffers), since proofs and keys
// have no fixed size.
package main

/*
#include "tint.h"
#include <stdlib.h>
*/
import "C"

import "unsafe"

func main() {}

// FreeBytes frees a buffer TintProve wrote into one of its out
// parameters.
//
//export FreeBytes
func FreeBytes(ptr *C.uint8_t) {
	if ptr == nil {
		return
	}
	C.free(unsafe.Pointer(ptr))
}
