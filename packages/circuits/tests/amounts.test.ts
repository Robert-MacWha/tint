import { WitnessTester } from "circomkit";
import { describe } from "mocha";
import { circomkit, randomBigInt } from "./common";
import { poseidon3 } from "poseidon-lite";

describe("CheckUnshields", () => {
    let circuit: WitnessTester<["commitmentsOut", "unshieldAmounts", "unshieldAssets", "assetsOut", "amountsOut"], []>;

    before(async () => {
        circuit = await circomkit.WitnessTester(`amounts`, {
            file: "amounts",
            template: "CheckUnshields",
            pubs: ["commitmentsOut", "unshieldAmounts", "unshieldAssets", "assetsOut", "amountsOut"],
            params: [2],
        });
    });

    it("should pass when all unshields are zero", async () => {
        const input = {
            commitmentsOut: [randomBigInt(), randomBigInt()],
            unshieldAmounts: [0n, 0n],
            unshieldAssets: [0n, 0n],
            assetsOut: [randomBigInt(), randomBigInt()],
            amountsOut: [randomBigInt(), randomBigInt()],
        };

        await circuit.expectPass(input);
    });

    it("should pass when unshields are non-zero and match outputs", async () => {
        const [asset1, amount1] = [randomBigInt(), randomBigInt()];

        const input = {
            commitmentsOut: [0, randomBigInt()],
            unshieldAmounts: [amount1, 0n],
            unshieldAssets: [asset1, 0n],
            assetsOut: [asset1, randomBigInt()],
            amountsOut: [amount1, randomBigInt()],
        };

        await circuit.expectPass(input);
    });

    it("should fail when unshield assets do not match outputs", async () => {
        const [asset1, amount1] = [randomBigInt(), randomBigInt()];

        const input = {
            commitmentsOut: [0, randomBigInt()],
            unshieldAmounts: [randomBigInt(), 0n],
            unshieldAssets: [asset1, 0n],
            assetsOut: [amount1, randomBigInt()],
            amountsOut: [amount1 + 1n, randomBigInt()],
        };
    });

    it("should fail when unshield amounts do not match outputs", async () => {
        const [asset1, amount1] = [randomBigInt(), randomBigInt()];

        const input = {
            commitmentsOut: [0, randomBigInt()],
            unshieldAmounts: [amount1, 0n],
            unshieldAssets: [asset1, 0n],
            assetsOut: [asset1, randomBigInt()],
            amountsOut: [amount1 + 1n, randomBigInt()],
        };

        await circuit.expectFail(input);
    });

    it("should fail when commitmentsOut and unshieldAmounts are non-zero", async () => {
        const [asset1, amount1] = [randomBigInt(), randomBigInt()];

        const input = {
            commitmentsOut: [randomBigInt(), randomBigInt()],
            unshieldAmounts: [amount1, 0n],
            unshieldAssets: [asset1, 0n],
            assetsOut: [asset1, randomBigInt()],
            amountsOut: [amount1, randomBigInt()],
        };

        await circuit.expectFail(input);
    });

    it("should fail when commitmentsOut and unshieldAssets are non-zero", async () => {
        const [asset1, amount1] = [randomBigInt(), randomBigInt()];

        const input = {
            commitmentsOut: [randomBigInt(), randomBigInt()],
            unshieldAmounts: [amount1, 0n],
            unshieldAssets: [asset1, 0n],
            assetsOut: [asset1, randomBigInt()],
            amountsOut: [amount1, randomBigInt()],
        };

        await circuit.expectFail(input);
    });
});

describe("CheckInputsBalanceOutputs", () => {
    let circuit: WitnessTester<["assetsIn", "amountsIn", "assetsOut", "amountsOut"], []>;

    before(async () => {
        circuit = await circomkit.WitnessTester(`amounts`, {
            file: "amounts",
            template: "CheckInputsBalanceOutputs",
            pubs: ["assetsIn", "amountsIn", "assetsOut", "amountsOut"],
            params: [2, 2],
        });
    });

    it("should pass when input and output assets and amounts match", async () => {
        const [asset1, asset2] = [randomBigInt(), randomBigInt()];
        const [amount1, amount2] = [randomBigInt(), randomBigInt()];

        const input = {
            assetsIn: [asset1, asset2],
            amountsIn: [amount1, amount2],
            assetsOut: [asset1, asset2],
            amountsOut: [amount1, amount2],
        };

        await circuit.expectPass(input);
    });

    it("should pass when input and output amounts match for input assets", async () => {
        // This is an intentional limitation of the circuit: it only checks for each
        // input asset that the total input amount for that asset matches the total
        // output amount for that asset. Orphan output assets are disallowed by the 
        // `CheckNoOrphanOutputs` circuit.
        const [asset1, asset2] = [randomBigInt(), randomBigInt()];
        const [amount1, amount2] = [randomBigInt(), randomBigInt()];

        const input = {
            assetsIn: [asset1, asset1],
            amountsIn: [amount1, amount2],
            assetsOut: [asset1, asset2],
            amountsOut: [amount1 + amount2, randomBigInt()],
        };

        await circuit.expectPass(input);
    });

    it("should fail when input and output assets do not match", async () => {
        const [asset1, asset2] = [randomBigInt(), randomBigInt()];
        const [amount1, amount2] = [randomBigInt(), randomBigInt()];

        const input = {
            assetsIn: [asset1, asset2],
            amountsIn: [amount1, amount2],
            assetsOut: [asset1 + 1n, asset2],
            amountsOut: [amount1, amount2],
        };

        await circuit.expectFail(input);
    });

    it("should fail when input and output amounts do not match", async () => {
        const [asset1, asset2] = [randomBigInt(), randomBigInt()];
        const [amount1, amount2] = [randomBigInt(), randomBigInt()];

        const input = {
            assetsIn: [asset1, asset2],
            amountsIn: [amount1, amount2],
            assetsOut: [asset1, asset2],
            amountsOut: [amount1 + 1n, amount2],
        };

        await circuit.expectFail(input);
    });
});

describe("CheckNoOrphanOutputs", () => {
    let circuit: WitnessTester<["assetsIn", "assetsOut"], []>;

    before(async () => {
        circuit = await circomkit.WitnessTester(`amounts`, {
            file: "amounts",
            template: "CheckNoOrphanOutputs",
            pubs: ["assetsIn", "assetsOut"],
            params: [2, 2],
        });
    });

    it("should pass when all output assets have a matching input asset", async () => {
        const [asset1, asset2] = [randomBigInt(), randomBigInt()];

        const input = {
            assetsIn: [asset1, asset2],
            assetsOut: [asset1, asset2],
        };

        await circuit.expectPass(input);
    });

    it("should fail when an output asset does not have a matching input asset", async () => {
        const [asset1, asset2] = [randomBigInt(), randomBigInt()];

        const input = {
            assetsIn: [asset1, asset2],
            assetsOut: [asset1, asset2 + 1n],
        };

        await circuit.expectFail(input);
    });
});
