import { WitnessTester } from "circomkit";
import { describe } from "mocha";
import { circomkit } from "./common";
import { poseidon4 } from "poseidon-lite";

// depth=2: 4^2 = 16 leaves, 2 levels of hashing
const DEPTH = 2;
const LEAVES = Array.from({ length: 16 }, (_, i) => BigInt(i + 1));

// Level-1 group hashes (4 groups of 4 leaves)
const L1 = [
    poseidon4(LEAVES.slice(0, 4)),
    poseidon4(LEAVES.slice(4, 8)),
    poseidon4(LEAVES.slice(8, 12)),
    poseidon4(LEAVES.slice(12, 16)),
];
const ROOT = poseidon4(L1);

describe("BatchMerkleRoot4", () => {
    let circuit: WitnessTester<["leaves"], ["root"]>;

    before(async () => {
        circuit = await circomkit.WitnessTester("batchMerkleRoot4", {
            file: "batchMerkleTree4",
            template: "BatchMerkleRoot4",
            pubs: ["leaves"],
            params: [DEPTH],
        });
    });

    it("should compute the correct root", async () => {
        await circuit.expectPass({ leaves: LEAVES }, { root: ROOT });
    });
});

describe("BatchMerkleInclusion4", () => {
    let circuit: WitnessTester<["leaf", "leafIndex", "siblings"], ["root"]>;

    before(async () => {
        circuit = await circomkit.WitnessTester("batchMerkleInclusion4", {
            file: "batchMerkleTree4",
            template: "BatchMerkleInclusion4",
            pubs: ["leaf", "leafIndex", "siblings"],
            params: [DEPTH],
        });
    });

    for (let i = 0; i < 16; i++) {
        it(`should verify inclusion at leafIndex ${i}`, async () => {
            const group = Math.floor(i / 4);
            const pos = i % 4;
            const lvl0 = [0, 1, 2, 3].filter(k => k !== pos).map(k => LEAVES[group * 4 + k]);
            const lvl1 = [0, 1, 2, 3].filter(k => k !== group).map(k => L1[k]);
            await circuit.expectPass(
                { leaf: LEAVES[i], leafIndex: BigInt(i), siblings: [lvl0, lvl1] },
                { root: ROOT },
            );
        });
    }
});
