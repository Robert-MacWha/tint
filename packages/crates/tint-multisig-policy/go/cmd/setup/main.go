// Command setup generates the MultisigPolicyCircuit's and
// HardwareWalletPolicyCircuit's Groth16 setup artifacts and writes them to
// ../artifacts and ../artifacts/hardware_wallet respectively.
package main

import (
	"fmt"
	"io"
	"log"
	"os"

	"github.com/Robert-MacWha/tint/packages/crates/tint-multisig-policy/go/circuit"
	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
	gnarkio "github.com/consensys/gnark/io"
	"github.com/consensys/gnark/std/math/emulated"
)

const (
	multisigDir       = "../artifacts/"
	hardwareWalletDir = "../artifacts/hardware_wallet/"
)

func main() {
	var multisig circuit.MultisigPolicyCircuit[emulated.Secp256k1Fp, emulated.Secp256k1Fr]
	if err := generateArtifacts(&multisig, multisigDir); err != nil {
		log.Fatal(err)
	}
	var hardwareWallet circuit.HardwareWalletPolicyCircuit[emulated.Secp256k1Fp, emulated.Secp256k1Fr]
	if err := generateArtifacts(&hardwareWallet, hardwareWalletDir); err != nil {
		log.Fatal(err)
	}
}

func generateArtifacts(c frontend.Circuit, artifactsDir string) error {
	fmt.Println("Generating setup artifacts in", artifactsDir)

	ccs, err := frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, c)
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
