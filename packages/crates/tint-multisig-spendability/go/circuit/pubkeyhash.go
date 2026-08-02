// Package circuit implements the MultisigSpendability gnark circuit.
package circuit

import (
	"math/big"

	frbn254 "github.com/consensys/gnark-crypto/ecc/bn254/fr"
	"github.com/consensys/gnark/frontend"
	stdbits "github.com/consensys/gnark/std/math/bits"
	"github.com/consensys/gnark/std/math/emulated"
	stdecdsa "github.com/consensys/gnark/std/signature/ecdsa"

	"github.com/Robert-MacWha/tint/packages/crates/tint-multisig-spendability/go/internal/poseidon2"
)

// PubKeyXY is a secp256k1 public key's raw (X,Y) coordinates, each a
// ~256-bit big-endian integer.
type PubKeyXY struct {
	X, Y *big.Int
}

var twoTo128 = new(big.Int).Lsh(big.NewInt(1), 128)

// PubKeyHash is the native (out-of-circuit) counterpart of PubKeyHashGadget:
// a running T=2 poseidon2 accumulator over each pubkey's [Xhi,Xlo,Yhi,Ylo]
// 128-bit limbs, in signer order — the same running-accumulator idiom
// operation.Operation.Hash already uses.
func PubKeyHash(pubKeys []PubKeyXY) frbn254.Element {
	var hash frbn254.Element
	for _, pk := range pubKeys {
		xHi, xLo := split128(pk.X)
		yHi, yLo := split128(pk.Y)
		for _, limb := range []*big.Int{xHi, xLo, yHi, yLo} {
			hash = poseidon2.Compress2([2]frbn254.Element{hash, limbToFr(limb)})
		}
	}
	return hash
}

// PubKeyHashGadget is the in-circuit counterpart of PubKeyHash. Each
// coordinate is an emulated secp256k1 base-field element; ToBitsCanonical is
// the implementation-independent way to export it into native-field bits
// (unlike reading an emulated.Element's internal .Limbs, whose width/count
// is an implementation detail of the field's configuration, not a canonical
// value). secp256k1's Fp modulus is a full ~256-bit prime (not a power of
// two), so the two-limb split has no truncation edge case.
func PubKeyHashGadget[Base, Scalar emulated.FieldParams](
	api frontend.API,
	baseField *emulated.Field[Base],
	pubKeys []stdecdsa.PublicKey[Base, Scalar],
) frontend.Variable {
	var hash frontend.Variable = 0
	for _, pk := range pubKeys {
		for _, limb := range coordinateLimbs(api, baseField, &pk.X) {
			hash = poseidon2.CompressGadget2(api, [2]frontend.Variable{hash, limb})
		}
		for _, limb := range coordinateLimbs(api, baseField, &pk.Y) {
			hash = poseidon2.CompressGadget2(api, [2]frontend.Variable{hash, limb})
		}
	}
	return hash
}

// split128 splits v (< 2^256) into big-endian 128-bit halves: v = hi*2^128 + lo.
func split128(v *big.Int) (hi, lo *big.Int) {
	hi, lo = new(big.Int), new(big.Int)
	hi.DivMod(v, twoTo128, lo)
	return hi, lo
}

func limbToFr(limb *big.Int) frbn254.Element {
	var e frbn254.Element
	e.SetBigInt(limb) // limb < 2^128, well within Fr's ~254-bit range: no reduction risk.
	return e
}

// coordinateLimbs returns [hi, lo] such that the coordinate's canonical
// value equals hi*2^128 + lo.
func coordinateLimbs[Base emulated.FieldParams](api frontend.API, baseField *emulated.Field[Base], x *emulated.Element[Base]) [2]frontend.Variable {
	bits := baseField.ToBitsCanonical(x) // LSB-first, 256 bits for secp256k1 Fp.
	lo := stdbits.FromBinary(api, bits[0:128])
	hi := stdbits.FromBinary(api, bits[128:256])
	return [2]frontend.Variable{hi, lo}
}
