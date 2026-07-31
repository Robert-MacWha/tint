package main

import (
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/sha256"
	"fmt"
	"math/big"
	"time"

	"github.com/consensys/gnark-crypto/ecc"
	k1ecdsa "github.com/consensys/gnark-crypto/ecc/secp256k1/ecdsa"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
	"github.com/consensys/gnark/std/algebra/algopts"
	"github.com/consensys/gnark/std/algebra/emulated/sw_emulated"
	"github.com/consensys/gnark/std/math/emulated"
	stdecdsa "github.com/consensys/gnark/std/signature/ecdsa"
	"golang.org/x/crypto/cryptobyte"
	"golang.org/x/crypto/cryptobyte/asn1"
)

type EcdsaCircuit[T, S emulated.FieldParams] struct {
	Sig stdecdsa.Signature[S]
	Msg emulated.Element[S]
	Pub stdecdsa.PublicKey[T, S]
}

func (c *EcdsaCircuit[T, S]) Define(api frontend.API) error {
	c.Pub.Verify(api, sw_emulated.GetCurveParams[T](), &c.Msg, &c.Sig, algopts.WithIncompleteArithmetic())
	return nil
}

func main() {
	benchFieldMul()
	benchSecp256k1()
	benchP256()
}

// FieldMulCircuit isolates a single non-native multiply, for comparison
// against arkworks' EmulatedFpVar (see
// tint-multisig-spendability/tests/nonnative_bench.rs) independent of any
// curve/scalar-mult algorithm choice.
type FieldMulCircuit[T emulated.FieldParams] struct {
	A, B emulated.Element[T]
}

func (c *FieldMulCircuit[T]) Define(api frontend.API) error {
	f, err := emulated.NewField[T](api)
	if err != nil {
		return err
	}
	f.Mul(&c.A, &c.B)
	return nil
}

func benchFieldMul() {
	circuit := FieldMulCircuit[emulated.Secp256k1Fp]{}
	ccs, err := frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)
	if err != nil {
		panic(err)
	}
	fmt.Printf("single secp256k1 Fp multiply: %d R1CS constraints\n\n", ccs.GetNbConstraints())
}

// benchSecp256k1 signs with gnark-crypto's secp256k1 (the curve Ethereum
// wallets use) and verifies the signature in-circuit.
func benchSecp256k1() {
	privKey, err := k1ecdsa.GenerateKey(rand.Reader)
	if err != nil {
		panic(err)
	}
	publicKey := privKey.PublicKey

	msg := []byte("testing ECDSA (pre-hashed)")
	sigBin, err := privKey.Sign(msg, nil)
	if err != nil {
		panic(err)
	}
	if ok, err := publicKey.Verify(sigBin, msg, nil); err != nil || !ok {
		panic("signature is not valid")
	}

	var sig k1ecdsa.Signature
	sig.SetBytes(sigBin)
	r, s := new(big.Int), new(big.Int)
	r.SetBytes(sig.R[:32])
	s.SetBytes(sig.S[:32])
	hash := k1ecdsa.HashToInt(msg)

	pubX, pubY := new(big.Int), new(big.Int)
	publicKey.A.X.BigInt(pubX)
	publicKey.A.Y.BigInt(pubY)

	bench[emulated.Secp256k1Fp, emulated.Secp256k1Fr]("secp256k1", r, s, hash, pubX, pubY)
}

// benchP256 signs with the standard library's P-256 (secp256r1) and verifies
// the signature in-circuit.
func benchP256() {
	privKey, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	if err != nil {
		panic(err)
	}
	publicKey := privKey.PublicKey

	msg := []byte("testing ECDSA (pre-hashed)")
	msgHash := sha256.Sum256(msg)
	sigBin, err := ecdsa.SignASN1(rand.Reader, privKey, msgHash[:])
	if err != nil {
		panic(err)
	}
	if !ecdsa.VerifyASN1(&publicKey, msgHash[:], sigBin) {
		panic("signature is not valid")
	}

	var (
		r, s  = new(big.Int), new(big.Int)
		inner cryptobyte.String
	)
	input := cryptobyte.String(sigBin)
	if !input.ReadASN1(&inner, asn1.SEQUENCE) ||
		!input.Empty() ||
		!inner.ReadASN1Integer(r) ||
		!inner.ReadASN1Integer(s) ||
		!inner.Empty() {
		panic("invalid signature encoding")
	}
	hash := new(big.Int).SetBytes(msgHash[:])

	bench[emulated.P256Fp, emulated.P256Fr]("secp256r1", r, s, hash, publicKey.X, publicKey.Y)
}

// bench compiles, sets up, proves, and verifies an ECDSA-verification
// circuit over the given curve/scalar field, timing each stage.
func bench[T, S emulated.FieldParams](name string, r, s, hash, pubX, pubY *big.Int) {
	fmt.Printf("=== %s ===\n", name)

	circuit := EcdsaCircuit[T, S]{}
	witness := EcdsaCircuit[T, S]{
		Sig: stdecdsa.Signature[S]{
			R: emulated.ValueOf[S](r),
			S: emulated.ValueOf[S](s),
		},
		Msg: emulated.ValueOf[S](hash),
		Pub: stdecdsa.PublicKey[T, S]{
			X: emulated.ValueOf[T](pubX),
			Y: emulated.ValueOf[T](pubY),
		},
	}

	t0 := time.Now()
	ccs, err := frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)
	if err != nil {
		panic(err)
	}
	fmt.Printf("compile: %s (%d constraints)\n", time.Since(t0), ccs.GetNbConstraints())

	t0 = time.Now()
	pk, vk, err := groth16.Setup(ccs)
	if err != nil {
		panic(err)
	}
	fmt.Printf("setup:   %s\n", time.Since(t0))

	fullWitness, err := frontend.NewWitness(&witness, ecc.BN254.ScalarField())
	if err != nil {
		panic(err)
	}
	publicWitness, err := fullWitness.Public()
	if err != nil {
		panic(err)
	}

	t0 = time.Now()
	proof, err := groth16.Prove(ccs, pk, fullWitness)
	if err != nil {
		panic(err)
	}
	fmt.Printf("prove:   %s\n", time.Since(t0))

	t0 = time.Now()
	if err := groth16.Verify(proof, vk, publicWitness); err != nil {
		panic(err)
	}
	fmt.Printf("verify:  %s\n\n", time.Since(t0))
}
