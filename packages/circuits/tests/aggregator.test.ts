import { circomkit } from "./common";
import {
  buildAggregatorInput,
  InputPlacement,
  Output,
  ARCHIVE_DEPTH,
  STAGING_DEPTH,
  Note,
} from "./common/notes";
import type { WitnessTester } from "circomkit";

const ASSET_A = 1n;
const ASSET_B = 2n;
const ASSET_C = 3n;

const MAX_AMOUNT = 2n ** 128n - 1n; // largest value Check128 (Num2Bits(128)) accepts

/// Builds a distinct note so commitments/nullifiers never collide between inputs.
function note(asset: bigint, amount: bigint, seed: number): Note {
  return {
    asset,
    amount,
    random: BigInt(2000 + seed),
    nullifyingKey: BigInt(1000 + seed),
    spendabilityAddress: 3000n,
    spendabilityData: 4000n,
  };
}

function output(asset: bigint, amount: bigint, seed: number): Output {
  return { asset, amount, partial: BigInt(500_000 + seed) };
}

/// A balanced, all-staging 4-in / 4-out transaction used as the happy path and
/// as the starting point that negative tests mutate.
function baseInputs(): InputPlacement[] {
  return [
    { ...note(ASSET_A, 100n, 0), tree: "staging", index: 0 },
    { ...note(ASSET_A, 50n, 1), tree: "staging", index: 1 },
    { ...note(ASSET_B, 30n, 2), tree: "staging", index: 2 },
    { ...note(ASSET_B, 70n, 3), tree: "staging", index: 3 },
  ];
}

function baseOutputs(): Output[] {
  return [
    output(ASSET_A, 80n, 0), // A: 80 + 70 = 150 == 100 + 50
    output(ASSET_A, 70n, 1),
    output(ASSET_B, 60n, 2), // B: 60 + 40 = 100 == 30 + 70
    output(ASSET_B, 40n, 3),
  ];
}

describe("Aggregator", () => {
  let circuit: WitnessTester<string[], string[]>;

  before(async function () {
    this.timeout(300_000);
    circuit = await circomkit.WitnessTester("aggregator", {
      file: "aggregator",
      template: "Aggregator",
      params: [4, 4, STAGING_DEPTH, ARCHIVE_DEPTH],
      pubs: ["stagingRoot", "archiveRoot", "nullifiers", "commitmentsOut", "unshieldRecipients", "unshieldAmounts", "unshieldAssets", "spendabilityAddressesIn", "spendabilityDataIn"],
      recompile: true,
    });
  });

  it("accepts a balanced, well-formed transaction", async () => {
    const input = buildAggregatorInput(baseInputs(), baseOutputs());
    await circuit.expectPass(input);
  });

  it("accepts an input spent from the archive tree", async () => {
    const inputs: InputPlacement[] = [
      { ...note(ASSET_A, 100n, 0), tree: "archive", index: 5 },
      { ...note(ASSET_A, 50n, 1), tree: "staging", index: 0 },
      { ...note(ASSET_B, 30n, 2), tree: "staging", index: 1 },
      { ...note(ASSET_B, 70n, 3), tree: "archive", index: 9 },
    ];
    await circuit.expectPass(buildAggregatorInput(inputs, baseOutputs()));
  });

  // Regression for the shared-leaf-index fix: previously the staging Num2Bits(8)
  // bounded the (shared) index to < 256, making any archive note at index >= 256
  // unspendable. Independent indices must now allow it.
  it("accepts an archive note at index >= 256", async () => {
    const inputs: InputPlacement[] = [
      { ...note(ASSET_A, 100n, 0), tree: "archive", index: 256 },
      { ...note(ASSET_A, 0n, 1), tree: "staging", index: 0 },
      { ...note(ASSET_A, 0n, 2), tree: "staging", index: 1 },
      { ...note(ASSET_A, 0n, 3), tree: "staging", index: 2 },
    ];
    const outputs: Output[] = [
      output(ASSET_A, 100n, 0),
      output(ASSET_A, 0n, 1),
      output(ASSET_A, 0n, 2),
      output(ASSET_A, 0n, 3),
    ];
    await circuit.expectPass(buildAggregatorInput(inputs, outputs));
  });

  it("accepts the maximum 128-bit amount (2^128 - 1)", async () => {
    const inputs: InputPlacement[] = [
      { ...note(ASSET_A, MAX_AMOUNT, 0), tree: "staging", index: 0 },
      { ...note(ASSET_A, 0n, 1), tree: "staging", index: 1 },
      { ...note(ASSET_A, 0n, 2), tree: "staging", index: 2 },
      { ...note(ASSET_A, 0n, 3), tree: "staging", index: 3 },
    ];
    const outputs: Output[] = [
      output(ASSET_A, MAX_AMOUNT, 0),
      output(ASSET_A, 0n, 1),
      output(ASSET_A, 0n, 2),
      output(ASSET_A, 0n, 3),
    ];
    await circuit.expectPass(buildAggregatorInput(inputs, outputs));
  });

  it("accepts an unshield output", async () => {
    const input = buildAggregatorInput(baseInputs(), baseOutputs());
    // Convert output 0 (ASSET_A, 80n) to an unshield
    input.commitmentsOut[0] = 0n;
    input.unshieldRecipients[0] = 99999n;
    input.amountsOut[0] = 80n;
    input.unshieldAmounts[0] = 80n;
    input.unshieldAssets[0] = ASSET_A;
    await circuit.expectPass(input);
  });

  it("rejects an unshield with a mismatched public amount", async () => {
    const input = buildAggregatorInput(baseInputs(), baseOutputs());
    input.commitmentsOut[0] = 0n;
    input.unshieldRecipients[0] = 99999n;
    input.amountsOut[0] = 80n;
    input.unshieldAmounts[0] = 81n;
    input.unshieldAssets[0] = ASSET_A;
    await circuit.expectFail(input);
  });

  // Overflow guard: an amount of exactly 2^128 must be rejected by Check128.
  // The transaction is otherwise valid and balanced, so the range check is the
  // only violated constraint.
  it("rejects an amount of 2^128 (overflow guard)", async () => {
    const overflow = 2n ** 128n;
    const inputs: InputPlacement[] = [
      { ...note(ASSET_A, overflow, 0), tree: "staging", index: 0 },
      { ...note(ASSET_A, 0n, 1), tree: "staging", index: 1 },
      { ...note(ASSET_A, 0n, 2), tree: "staging", index: 2 },
      { ...note(ASSET_A, 0n, 3), tree: "staging", index: 3 },
    ];
    const outputs: Output[] = [
      output(ASSET_A, overflow, 0),
      output(ASSET_A, 0n, 1),
      output(ASSET_A, 0n, 2),
      output(ASSET_A, 0n, 3),
    ];
    await circuit.expectFail(buildAggregatorInput(inputs, outputs));
  });

  it("rejects a transaction that creates value (unbalanced outputs)", async () => {
    const outputs = baseOutputs();
    outputs[3] = output(ASSET_B, 41n, 3); // B out = 60 + 41 = 101 != 100
    await circuit.expectFail(buildAggregatorInput(baseInputs(), outputs));
  });

  it("rejects an output whose asset has no matching input (orphan)", async () => {
    const outputs: Output[] = [
      output(ASSET_A, 150n, 0), // A balances
      output(ASSET_B, 100n, 1), // B balances
      output(ASSET_C, 5n, 2), // orphan asset, created from nothing
      output(ASSET_C, 0n, 3),
    ];
    await circuit.expectFail(buildAggregatorInput(baseInputs(), outputs));
  });

  it("rejects a mismatched nullifier", async () => {
    const input = buildAggregatorInput(baseInputs(), baseOutputs());
    input.nullifiers[0] = input.nullifiers[0] + 1n;
    await circuit.expectFail(input);
  });

  it("rejects a note absent from both trees", async () => {
    const input = buildAggregatorInput(baseInputs(), baseOutputs());
    // Corrupt the staging proof so neither the staging nor the (empty) archive
    // branch reproduces its root.
    input.stagingPathElementsIn[0][0] = input.stagingPathElementsIn[0][0] + 1n;
    await circuit.expectFail(input);
  });
});
