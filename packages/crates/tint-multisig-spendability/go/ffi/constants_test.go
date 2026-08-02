package main

import (
	"testing"

	"github.com/Robert-MacWha/tint/packages/crates/tint-multisig-spendability/go/circuit"
	"github.com/Robert-MacWha/tint/packages/crates/tint-multisig-spendability/go/internal/operation"
)

func TestConstants(t *testing.T) {
	if operation.NInputs != FfiNInputs {
		t.Errorf("NInputs = %d; want %d", operation.NInputs, FfiNInputs)
	}
	if operation.NOutputs != FfiNOutputs {
		t.Errorf("NOutputs = %d; want %d", operation.NOutputs, FfiNOutputs)
	}
	if operation.NWithdrawals != FfiNWithdrawals {
		t.Errorf("NWithdrawals = %d; want %d", operation.NWithdrawals, FfiNWithdrawals)
	}
	if circuit.NSigners != FfiNSigners {
		t.Errorf("NSigners = %d; want %d", circuit.NSigners, FfiNSigners)
	}
}
