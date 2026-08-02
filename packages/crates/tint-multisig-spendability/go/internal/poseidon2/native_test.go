package poseidon2

import (
	"testing"

	fr "github.com/consensys/gnark-crypto/ecc/bn254/fr"
)

func frOf(v uint64) fr.Element {
	var e fr.Element
	e.SetUint64(v)
	return e
}

func TestCompressVectors(t *testing.T) {
	tests := []struct {
		name string
		got  fr.Element
		want string
	}{
		{
			"compress1(1)",
			Compress1(frOf(1)),
			"4220003009428892662276135118827607177546592752204629865937061707152838643029",
		},
		{
			"compress2([1,2])",
			Compress2([2]fr.Element{frOf(1), frOf(2)}),
			"6588139247708940112588203339651261153905233202198520634825199962343944922547",
		},
		{
			"compress3([1,2,3])",
			Compress3([3]fr.Element{frOf(1), frOf(2), frOf(3)}),
			"4737982494702600552753609419126955242994596445692557044681458296415162795881",
		},
		{
			"compress8([1..8])",
			Compress8([8]fr.Element{frOf(1), frOf(2), frOf(3), frOf(4), frOf(5), frOf(6), frOf(7), frOf(8)}),
			"1560309679398480135637530296453394701947175812702002696334681214336713681548",
		},
		{
			"compress2([0,0])",
			Compress2([2]fr.Element{frOf(0), frOf(0)}),
			"15621590199821056450610068202457788725601603091791048810523422053872049975191",
		},
	}

	for _, tc := range tests {
		if got := tc.got.String(); got != tc.want {
			t.Errorf("%s = %s, want %s", tc.name, got, tc.want)
		}
	}
}
