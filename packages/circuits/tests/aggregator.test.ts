import { WitnessTester } from "circomkit";
import { describe, before, it } from "mocha";
import { circomkit } from "./common";
import {
    buildAggregatorInput,
    N_INPUTS, N_OUTPUTS, INNER_DEPTH, OUTER_DEPTH,
    type Note, type Output,
} from "./common/notes";

const ASSET_A = 1n;
const ASSET_B = 2n;
const ASSET_C = 3n;

const note = (asset: bigint, amount: bigint, seed: number): Note => ({
    asset, amount,
    nullifyingKey: BigInt(1000 + seed),
    partialCommitment: BigInt(500_000 + seed),
});

const output = (asset: bigint, amount: bigint, seed: number): Output => ({
    asset, amount, partialCommitment: BigInt(900_000 + seed),
});

const baseInputs = (): (Note | null)[] => [note(ASSET_A, 100n, 0), note(ASSET_B, 50n, 1)];
const baseOutputs = (): Output[] => [output(ASSET_A, 100n, 0), output(ASSET_B, 50n, 1)];

describe("Aggregator", () => {
    let circuit: WitnessTester<string[], string[]>;

    before(async function () {
        this.timeout(30_000);
        circuit = await circomkit.WitnessTester("aggregator", {
            file: "aggregator",
            template: "Aggregator",
            params: [N_INPUTS, N_OUTPUTS, INNER_DEPTH, OUTER_DEPTH],
            pubs: [
                "oldRoot", "newRoot", "leavesAggregationHash", "nullifiers",
                "commitmentsOut", "unshieldAmounts", "unshieldAssets", "boundParamsHash",
            ],
        });
    });

    it("should pass for a balanced transaction", async () => {
        await circuit.expectPass(buildAggregatorInput(baseInputs(), baseOutputs()));
    });

    it("should pass for a balanced transaction with archive-tree input (batchIndex=1)", async () => {
        // batchIndex=1 puts the batch root at outer slot 1, exercising the Mux at a
        // non-zero position in the outer level, and shifts leaf indices to [4, 5].
        await circuit.expectPass(buildAggregatorInput(baseInputs(), baseOutputs(), 1));
    });

    it("should fail for amount mismatch", async () => {
        const inp = buildAggregatorInput(baseInputs(), baseOutputs());
        inp.amountsOut[0] += 1n;
        await circuit.expectFail(inp);
    });

    it("should fail for asset mismatch (orphan output)", async () => {
        await circuit.expectFail(buildAggregatorInput(
            baseInputs(),
            [output(ASSET_A, 100n, 0), output(ASSET_C, 50n, 1)],
        ));
    });

    it("should fail for wrong nullifier preimage", async () => {
        const inp = buildAggregatorInput(baseInputs(), baseOutputs());
        inp.nullifiers[0] += 1n;
        await circuit.expectFail(inp);
    });

    it("should fail for wrong Merkle path", async () => {
        const inp = buildAggregatorInput(baseInputs(), baseOutputs());
        inp.siblingsIn[0][0][0] += 1n;
        await circuit.expectFail(inp);
    });

    it("should fail for wrong newRoot", async () => {
        const inp = buildAggregatorInput(baseInputs(), baseOutputs());
        inp.newRoot += 1n;
        await circuit.expectFail(inp);
    });

    it("should fail for wrong leavesAggregationHash", async () => {
        const inp = buildAggregatorInput(baseInputs(), baseOutputs());
        inp.leavesAggregationHash += 1n;
        await circuit.expectFail(inp);
    });

    it("should pass for dummy slot correctly ignored", async () => {
        await circuit.expectPass(buildAggregatorInput(
            [note(ASSET_A, 100n, 0), null],
            [output(ASSET_A, 60n, 0), output(ASSET_A, 40n, 1)],
        ));
    });

    it("should fail for dummy slot with non-zero asset", async () => {
        const inp = buildAggregatorInput(
            [note(ASSET_A, 100n, 0), null],
            [output(ASSET_A, 60n, 0), output(ASSET_A, 40n, 1)],
        );
        inp.assetsIn[1] = ASSET_A; // violates: isDummy * assetsIn === 0
        await circuit.expectFail(inp);
    });

    it("should pass for boundParamsHash as a committed public signal", async () => {
        const inp = buildAggregatorInput(baseInputs(), baseOutputs());
        inp.boundParamsHash = 42n;
        await circuit.expectPass(inp);
    });
});
