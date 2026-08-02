package main

/*
#include "tint.h"
*/
import "C"

const (
	FfiNInputs      = int(C.N_INPUTS)
	FfiNOutputs     = int(C.N_OUTPUTS)
	FfiNWithdrawals = int(C.N_WITHDRAWALS)
	FfiNSigners     = int(C.N_SIGNERS)
)
