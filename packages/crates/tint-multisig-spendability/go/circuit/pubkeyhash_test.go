package circuit

import (
	"crypto/rand"
	"math/big"
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	k1ecdsa "github.com/consensys/gnark-crypto/ecc/secp256k1/ecdsa"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/std/math/emulated"
	stdecdsa "github.com/consensys/gnark/std/signature/ecdsa"
	"github.com/consensys/gnark/test"
)

const testNSigners = 2

type pubKeyHashCircuit struct {
	PubKeys  [testNSigners]stdecdsa.PublicKey[emulated.Secp256k1Fp, emulated.Secp256k1Fr]
	Expected frontend.Variable `gnark:",public"`
}

func (c *pubKeyHashCircuit) Define(api frontend.API) error {
	baseField, err := emulated.NewField[emulated.Secp256k1Fp](api)
	if err != nil {
		return err
	}
	api.AssertIsEqual(PubKeyHashGadget(api, baseField, c.PubKeys[:]), c.Expected)
	return nil
}

// TestPubKeyHashGadgetMatchesNative checks the in-circuit pubkey-set hash
// against the native PubKeyHash for real secp256k1 keypairs.
func TestPubKeyHashGadgetMatchesNative(t *testing.T) {
	var native [testNSigners]PubKeyXY
	var witness [testNSigners]stdecdsa.PublicKey[emulated.Secp256k1Fp, emulated.Secp256k1Fr]

	for i := range native {
		priv, err := k1ecdsa.GenerateKey(rand.Reader)
		if err != nil {
			t.Fatal(err)
		}
		x, y := new(big.Int), new(big.Int)
		priv.PublicKey.A.X.BigInt(x)
		priv.PublicKey.A.Y.BigInt(y)
		native[i] = PubKeyXY{X: x, Y: y}
		witness[i] = stdecdsa.PublicKey[emulated.Secp256k1Fp, emulated.Secp256k1Fr]{
			X: emulated.ValueOf[emulated.Secp256k1Fp](x),
			Y: emulated.ValueOf[emulated.Secp256k1Fp](y),
		}
	}

	want := PubKeyHash(native[:])

	assert := test.NewAssert(t)
	assert.CheckCircuit(&pubKeyHashCircuit{}, test.WithValidAssignment(&pubKeyHashCircuit{
		PubKeys:  witness,
		Expected: want.String(),
	}), test.WithCurves(ecc.BN254))
}

func TestSplit128RoundTrip(t *testing.T) {
	v, ok := new(big.Int).SetString("115792089237316195423570985008687907852837564279074904382605163141518161494337", 10) // secp256k1 Fp - 4 or similar large value
	if !ok {
		t.Fatal("bad test value")
	}
	hi, lo := split128(v)
	got := new(big.Int).Lsh(hi, 128)
	got.Add(got, lo)
	if got.Cmp(v) != 0 {
		t.Fatalf("split128 round trip: got %s, want %s", got, v)
	}
}
