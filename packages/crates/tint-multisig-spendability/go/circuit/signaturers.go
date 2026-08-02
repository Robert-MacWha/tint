package circuit

import "math/big"

type SignatureRS struct {
	R, S *big.Int
}
