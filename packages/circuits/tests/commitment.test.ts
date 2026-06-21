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
