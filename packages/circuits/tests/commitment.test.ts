import { WitnessTester } from "circomkit";
import { describe } from "mocha";
import { circomkit, randomBigInt } from "./common";
import { poseidon3, poseidon4 } from "poseidon-lite";

describe("CommitmentHasher", () => {
    let circuit: WitnessTester<["asset", "amount", "partialCommitment"], ["commitment"]>;

    before(async () => {
        circuit = await circomkit.WitnessTester(`commitment`, {
            file: "commitment",
            template: "CommitmentHasher",
            pubs: ["asset", "amount", "partialCommitment"],
        });
    });

    it("should compute the correct commitment", async () => {
        const input = {
            asset: randomBigInt(),
            amount: randomBigInt(),
            partialCommitment: randomBigInt(),
        };

        const commitmentHash = poseidon3([input.asset, input.amount, input.partialCommitment]);
        await circuit.expectPass(
            {
                asset: input.asset,
                amount: input.amount,
                partialCommitment: input.partialCommitment,
            },
            { commitment: commitmentHash },
        );

    });
});

describe("PartialCommitmentHasher", () => {
    let circuit: WitnessTester<["random", "nullifyingPubKey", "spendabilityAddress", "spendabilityData"], ["partialCommitment"]>;

    before(async () => {
        circuit = await circomkit.WitnessTester(`commitment`, {
            file: "commitment",
            template: "PartialCommitmentHasher",
            pubs: ["random", "nullifyingPubKey", "spendabilityAddress", "spendabilityData"],
        });
    });

    it("should compute the correct partial commitment", async () => {
        const input = {
            random: randomBigInt(),
            nullifyingPubKey: randomBigInt(),
            spendabilityAddress: randomBigInt(),
            spendabilityData: randomBigInt(),
        };

        const partialCommitmentHash = poseidon4([
            input.random,
            input.nullifyingPubKey,
            input.spendabilityAddress,
            input.spendabilityData,
        ]);

        await circuit.expectPass(
            {
                random: input.random,
                nullifyingPubKey: input.nullifyingPubKey,
                spendabilityAddress: input.spendabilityAddress,
                spendabilityData: input.spendabilityData,
            },
            { partialCommitment: partialCommitmentHash },
        );
    });
});