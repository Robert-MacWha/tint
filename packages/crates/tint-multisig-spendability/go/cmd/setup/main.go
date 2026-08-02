// Command setup generates the MultisigSpendability circuit's Groth16 setup
// artifacts and writes them to ../artifacts.
package main

import (
	"fmt"
	"io"
	"log"
	"os"

	"github.com/Robert-MacWha/tint/packages/crates/tint-multisig-spendability/go/circuit"
	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
	gnarkio "github.com/consensys/gnark/io"
	"github.com/consensys/gnark/std/math/emulated"
)

const artifactsDir = "../artifacts/"

func main() {
	err := generateArtifacts()
	if err != nil {
		log.Fatal(err)
	}
}

func generateArtifacts() error {
	fmt.Println("Generating setup artifacts")

	var c circuit.MultisigSpendability[emulated.Secp256k1Fp, emulated.Secp256k1Fr]
	ccs, err := frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &c)
	if err != nil {
		return fmt.Errorf("compiling circuit: %w", err)
	}

	pk, vk, err := groth16.Setup(ccs)
	if err != nil {
		return fmt.Errorf("running setup: %w", err)
	}

	if err := os.MkdirAll(artifactsDir, 0o755); err != nil {
		return fmt.Errorf("creating artifacts dir: %w", err)
	}
	writeFile(artifactsDir+"ccs.bin", ccs)
	writeDump(artifactsDir+"proving_key.bin", pk)
	writeFile(artifactsDir+"verifying_key.bin", vk)

	fmt.Println("Done generating setup artifacts")

	return nil
}

func writeFile(path string, w io.WriterTo) {
	fmt.Printf("Writing %s\n", path)
	f, err := os.Create(path)
	if err != nil {
		fmt.Fprintln(os.Stderr, "creating", path, ":", err)
		os.Exit(1)
	}
	defer f.Close()
	if _, err := w.WriteTo(f); err != nil {
		fmt.Fprintln(os.Stderr, "writing", path, ":", err)
		os.Exit(1)
	}
}

func writeDump(path string, w gnarkio.BinaryDumper) {
	fmt.Printf("Writing %s\n", path)
	f, err := os.Create(path)
	if err != nil {
		fmt.Fprintln(os.Stderr, "creating", path, ":", err)
		os.Exit(1)
	}
	defer f.Close()
	if err := w.WriteDump(f); err != nil {
		fmt.Fprintln(os.Stderr, "writing", path, ":", err)
		os.Exit(1)
	}
}
