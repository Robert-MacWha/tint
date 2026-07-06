import { WitnessTester } from "circomkit";
import { describe, before, it } from "mocha";
import { circomkit } from "./common";
import {
    buildAggregatorInput,
    N_INPUTS, N_OUTPUTS,
    type Note, type Output,
} from "./common/notes";

const BATCH_SIZE = 4;
const DEPTH = 3;

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
const base = () => buildAggregatorInput(baseInputs(), baseOutputs());

const dummyBase = () => buildAggregatorInput(
    [note(ASSET_A, 100n, 0), null],
    [output(ASSET_A, 60n, 0), output(ASSET_A, 40n, 1)],
);

describe("Aggregator", () => {
    let circuit: WitnessTester<string[], string[]>;

    before(async function () {
        this.timeout(30_000);
        circuit = await circomkit.WitnessTester("aggregator", {
            file: "aggregator",
            template: "Aggregator",
            params: [N_INPUTS, N_OUTPUTS, BATCH_SIZE, DEPTH],
            pubs: [
                "oldRoot", "newRoot", "startAggregationHash", "endAggregationHash",
                "nullifiers", "commitmentsOut", "unshieldAmounts", "unshieldAssets", "boundParamsHash",
            ],
        });
    });

    type Inp = ReturnType<typeof base>;
    const tamperFail = (mutate: (inp: Inp) => void) => async () => {
        const inp = base();
        mutate(inp);
        await circuit.expectFail(inp);
    };

    it("should pass for a balanced transaction", async () => {
        await circuit.expectPass(base());
    });

    it("should fail for amount mismatch",           tamperFail(inp => { inp.amountsOut[0] += 1n; }));
    it("should fail for wrong nullifier preimage",  tamperFail(inp => { inp.nullifiers[0] += 1n; }));
    it("should fail for wrong Merkle path",         tamperFail(inp => { inp.siblingsIn[0][0] += 1n; }));
    it("should fail for wrong newRoot",             tamperFail(inp => { inp.newRoot += 1n; }));
    it("should fail for wrong endAggregationHash",    tamperFail(inp => { inp.endAggregationHash += 1n; }));

    it("should fail for asset mismatch (orphan output)", async () => {
        await circuit.expectFail(buildAggregatorInput(
            baseInputs(),
            [output(ASSET_A, 100n, 0), output(ASSET_C, 50n, 1)],
        ));
    });

    it("should pass for dummy slot correctly ignored", async () => {
        await circuit.expectPass(dummyBase());
    });

    it("should fail for dummy slot with non-zero asset", async () => {
        const inp = dummyBase();
        inp.assetsIn[1] = ASSET_A; // violates: isDummy * assetsIn === 0
        await circuit.expectFail(inp);
    });

    it("should pass for boundParamsHash as a committed public signal", async () => {
        const inp = base();
        inp.boundParamsHash = 42n;
        await circuit.expectPass(inp);
    });
});
