import { WitnessTester } from "circomkit";
import { describe } from "mocha";
import { circomkit, randomBigInt, SimpleIMT } from "./common";

const INSERTIONS = 16;
const DEPTH = 24;

describe("MerkleTreeRootFromFrontier", () => {
    let circuit: WitnessTester<["frontier", "size"], ["out"]>;

    before(async () => {
        circuit = await circomkit.WitnessTester("merkleTreeRootFromFrontier", {
            file: "merkleTree",
            template: "MerkleTreeRootFromFrontier",
            pubs: ["frontier", "size"],
            params: [DEPTH],
        });
    });

    it("should compute the correct root from a frontier", async () => {
        const tree = new SimpleIMT(DEPTH);
        for (let i = 0; i < 10; i++) {
            tree.insert(randomBigInt());
        }

        await circuit.expectPass(
            { frontier: tree.frontier, size: BigInt(tree.size) },
            { out: tree.root },
        );
    });
});

describe("MerkleTreeBatchInsert", () => {
    let circuit: WitnessTester<["root", "startIndex", "leaves", "initialFrontier"], ["out"]>;

    before(async () => {
        circuit = await circomkit.WitnessTester("merkleTreeBatchInsert", {
            file: "merkleTree",
            template: "MerkleTreeBatchInsert",
            pubs: ["root", "startIndex", "leaves", "initialFrontier"],
            params: [DEPTH, INSERTIONS],
        });
    });

    it("should compute the correct new root when inserting a batch of leaves", async () => {
        const tree = new SimpleIMT(DEPTH);
        for (let i = 0; i < 10; i++) {
            tree.insert(randomBigInt());
        }

        const oldRoot = tree.root;
        const initialFrontier = tree.frontier;
        const leaves = Array.from({ length: INSERTIONS }, () => randomBigInt());
        const startIndex = BigInt(tree.size);

        for (const leaf of leaves) {
            tree.insert(leaf);
        }

        await circuit.expectPass(
            { startIndex, root: oldRoot, leaves, initialFrontier },
            { out: tree.root },
        );
    });
});

// Use a smaller depth for the inclusion test to avoid V8 JIT pressure
// after loading the large BatchInsert WASM in the same process.
// The circuit logic is identical at any depth — depth=8 is sufficient.
const INCLUSION_DEPTH = 8;

describe("MerkleTreeInclusion", () => {
    let circuit: WitnessTester<["leaf", "leafIndex", "siblings"], ["root"]>;

    before(async () => {
        circuit = await circomkit.WitnessTester("merkleTreeInclusion", {
            file: "merkleTree",
            template: "MerkleTreeInclusion",
            pubs: ["leaf", "leafIndex", "siblings"],
            params: [INCLUSION_DEPTH],
        });
    });

    it("should verify a valid inclusion proof", async () => {
        const tree = new SimpleIMT(INCLUSION_DEPTH);
        for (let i = 0; i < 10; i++) {
            tree.insert(randomBigInt());
        }

        const index = 5;
        const leaf = tree.leaves[index];
        const siblings = tree.generateProof(index);

        await circuit.expectPass(
            { leaf, leafIndex: BigInt(index), siblings },
            { root: tree.root },
        );
    });
});
