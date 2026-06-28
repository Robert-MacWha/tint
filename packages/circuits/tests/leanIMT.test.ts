import { WitnessTester } from "circomkit";
import { describe } from "mocha";
import { circomkit, randomBigInt, getFrontier } from "./common";
import { poseidon2 } from "poseidon-lite";
import { LeanIMT } from "@zk-kit/lean-imt";

const INSERTIONS = 16;
const DEPTH = 24;
const hash = (a: bigint, b: bigint) => poseidon2([a, b]);

describe("leanIMTRootFromFrontier", () => {
    let circuit: WitnessTester<["frontier", "size"], ["out"]>;

    before(async () => {
        circuit = await circomkit.WitnessTester("leanIMTRootFromFrontier", {
            file: "leanIMT",
            template: "LeanIMTRootFromFrontier",
            pubs: ["frontier", "size"],
            params: [DEPTH],
        });
    });

    it("should compute the correct root from a frontier", async () => {
        const tree = new LeanIMT(hash);
        for (let i = 0; i < 10; i++) {
            tree.insert(randomBigInt());
        }

        const frontier = getFrontier(tree, DEPTH);
        await circuit.expectPass({ frontier, size: BigInt(tree.size) }, { out: tree.root });
    });
});

describe("LeanIMTInsertLeaf", () => {
    let circuit: WitnessTester<["leafHash", "insertionIndex", "frontier"], ["newFrontier"]>;

    before(async () => {
        circuit = await circomkit.WitnessTester("leanIMTInsertLeaf", {
            file: "leanIMT",
            template: "LeanIMTInsertLeaf",
            pubs: ["leafHash", "insertionIndex", "frontier"],
            params: [DEPTH],
        });
    });

    it("should compute the correct new frontier when inserting a single leaf", async () => {
        const tree = new LeanIMT(hash);
        for (let i = 0; i < 10; i++) {
            tree.insert(randomBigInt());
        }

        const beforeFrontier = getFrontier(tree, DEPTH);
        const leafHash = randomBigInt();
        tree.insert(leafHash);
        const afterFrontier = getFrontier(tree, DEPTH);
        await circuit.expectPass({ leafHash, insertionIndex: BigInt(tree.size - 1), frontier: beforeFrontier }, { newFrontier: afterFrontier });
    });
})

describe("LeanIMTBatchInsert", () => {
    let circuit: WitnessTester<["root", "startIndex", "leaves", "initialFrontier"], ["out"]>;

    before(async () => {
        circuit = await circomkit.WitnessTester("leanIMTBatchInsert", {
            file: "leanIMT",
            template: "LeanIMTBatchInsert",
            pubs: ["root", "startIndex", "leaves", "initialFrontier"],
            params: [DEPTH, INSERTIONS],
        });
    });

    it("should compute the correct new root when inserting a batch of leaves", async () => {
        const tree = new LeanIMT(hash);
        for (let i = 0; i < 10; i++) {
            tree.insert(randomBigInt());
        }

        const oldRoot = tree.root;
        const initialFrontier = getFrontier(tree, DEPTH);
        const leaves = Array.from({ length: INSERTIONS }, () => randomBigInt());
        const startIndex = BigInt(tree.size);
        for (const leaf of leaves) {
            tree.insert(leaf);
        }
        const newRoot = tree.root;

        await circuit.expectPass({ startIndex, root: oldRoot, leaves, initialFrontier }, { out: newRoot });
    });
});
