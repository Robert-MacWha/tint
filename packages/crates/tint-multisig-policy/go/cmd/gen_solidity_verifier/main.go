// Command gen_solidity_verifier generates a Solidity verifier contract from the
// verifying key produced by the gnark circuit.
package main

import (
	"bytes"
	"fmt"
	"log"
	"os"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
)

const artifactsDir = "../artifacts/"

func main() {
	err := genSolidityVerifier()
	if err != nil {
		log.Fatal(err)
	}
}

func genSolidityVerifier() error {
	fmt.Println("Generating Solidity verifier")

	// Load the verifying key from file
	vk := groth16.NewVerifyingKey(ecc.BN254)
	data, err := os.ReadFile(artifactsDir + "verifying_key.bin")
	if err != nil {
		return fmt.Errorf("reading verifying key file: %w", err)
	}

	_, err = vk.ReadFrom(bytes.NewReader(data))
	if err != nil {
		return fmt.Errorf("decoding verifying key: %w", err)
	}

	// Generate the Solidity verifier code
	file, err := os.Create(artifactsDir + "verifier.sol")
	if err != nil {
		return fmt.Errorf("creating verifier.sol: %w", err)
	}
	defer file.Close()

	err = vk.ExportSolidity(file)
	if err != nil {
		return fmt.Errorf("exporting Solidity verifier: %w", err)
	}

	return nil
}
