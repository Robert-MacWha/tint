// Package main is the cgo/C ABI entry point for the multisig spendability
// circuit Go library.
package main

/*
#include "tint.h"
#include <stdlib.h>
*/
import "C"

import (
	"unsafe"

	"github.com/consensys/gnark/logger"
)

func main() {}

func init() {
	logger.Disable()
}

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
