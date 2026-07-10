import { WitnessTester } from "circomkit";
import { describe } from "mocha";
import { poseidon2 } from "poseidon-lite";
import { circomkit, randomBigInt, SimpleIMT } from "./common";

// Depth params kept small for fast WASM compilation in tests.
const D = 8;  // main tree depth
const d = 3;  // chunk depth: 2^d = 8 leaves per chunk

const chunkSize = 1 << d;

// --- Helpers ---

function padToSize(arr: bigint[], size: number): bigint[] {
    const out = [...arr];
    while (out.length < size) out.push(0n);
    return out;
}

// Siblings from chunk root up to tree root (path length = D - d).
function proveChunk(tree: SimpleIMT, chunkIndex: number): bigint[] {
    // All leaves in this chunk share the same upper-tree path.
    const leafIndex = chunkIndex * chunkSize;
    const fullProof = tree.generateProof(leafIndex);
    return fullProof.slice(d); // levels d..D-1
}

// Empty tree zeros: zeros[l] = Poseidon(zeros[l-1], zeros[l-1]).
function computeZeros(depth: number): bigint[] {
    const z = [0n];
    for (let l = 1; l <= depth; l++) z.push(poseidon2([z[l - 1], z[l - 1]]));
    return z;
}

// ---

describe("BatchMerkleRoot", () => {
    let circuit: WitnessTester<["leaves"], ["root"]>;

    before(async () => {
        circuit = await circomkit.WitnessTester("batchMerkleRoot_d3", {
            file: "batchMerkleTree",
            template: "BatchMerkleRoot",
            pubs: ["leaves"],
            params: [d],
        });
    });

    it("matches SimpleIMT for a full chunk", async () => {
        const leaves = Array.from({ length: chunkSize }, () => randomBigInt());
        const sub = new SimpleIMT(d);
        for (const l of leaves) sub.insert(l);
        await circuit.expectPass({ leaves }, { root: sub.root });
    });

    it("produces the correct zero root for an empty chunk", async () => {
        const leaves = new Array(chunkSize).fill(0n);
        const zeros = computeZeros(d);
        await circuit.expectPass({ leaves }, { root: zeros[d] });
    });

    it("matches SimpleIMT for a partially filled chunk", async () => {
        const leaves = new Array(chunkSize).fill(0n);
        for (let i = 0; i < 5; i++) leaves[i] = randomBigInt();
        const sub = new SimpleIMT(d);
        for (const l of leaves) sub.insert(l);
        await circuit.expectPass({ leaves }, { root: sub.root });
    });
});

describe("BatchMerkleInclusion", () => {
    let circuit: WitnessTester<["leaf", "leafIndex", "siblings"], ["root"]>;

    before(async () => {
        circuit = await circomkit.WitnessTester("batchMerkleInclusion_d3", {
            file: "batchMerkleTree",
            template: "BatchMerkleInclusion",
            pubs: ["leaf", "leafIndex", "siblings"],
            params: [d],
        });
    });

    it("proves a leaf is within its chunk", async () => {
        const sub = new SimpleIMT(d);
        for (let i = 0; i < chunkSize; i++) sub.insert(randomBigInt());
        const idx = 3;
        const siblings = sub.generateProof(idx);
        await circuit.expectPass(
            { leaf: sub.leaves[idx], leafIndex: BigInt(idx), siblings },
            { root: sub.root },
        );
    });
});

describe("BatchChunkInsert", () => {
    let circuit: WitnessTester<
        ["oldRoot", "newRoot", "currentChunkFilled", "currentChunkIndex",
         "existingLeaves", "newLeaves", "currentSiblings", "nextSiblings"],
        []
    >;

    before(async () => {
        circuit = await circomkit.WitnessTester("batchChunkInsert_D8_d3", {
            file: "batchMerkleTree",
            template: "BatchChunkInsert",
            pubs: ["oldRoot", "newRoot"],
            params: [D, d],
        });
    });

    it("inserts new leaves with no overflow", async () => {
        // Tree has 2 leaves. Current chunk (index 0) is 2/8 filled.
        const tree = new SimpleIMT(D);
        tree.insert(randomBigInt());
        tree.insert(randomBigInt());

        const filled = 2;
        const chunkIdx = 0;
        const oldRoot = tree.root;
        const existing = padToSize(tree.leaves.slice(0, chunkSize), chunkSize);

        // currentSiblings must be captured PRE-insert (siblings of chunk k in oldRoot).
        const currentSiblings = proveChunk(tree, chunkIdx);

        // Insert 3 new leaves (no overflow since 2+3=5 < 8).
        const newLeaves = [randomBigInt(), randomBigInt(), randomBigInt()];
        const paddedNew = padToSize(newLeaves, chunkSize);

        for (const l of newLeaves) tree.insert(l);
        const newRoot = tree.root;

        // nextSiblings captured POST-insert (siblings of chunk k+1 in the intermediate tree).
        const nextSiblings = proveChunk(tree, chunkIdx + 1);

        await circuit.expectPass({
            oldRoot,
            newRoot,
            currentChunkFilled: BigInt(filled),
            currentChunkIndex: BigInt(chunkIdx),
            existingLeaves: existing,
            newLeaves: paddedNew,
            currentSiblings,
            nextSiblings,
        });
    });

    it("inserts new leaves with overflow into next chunk", async () => {
        // Tree has 4 leaves in chunk 0.
        const tree = new SimpleIMT(D);
        for (let i = 0; i < 4; i++) tree.insert(randomBigInt());

        const filled = 4;
        const chunkIdx = 0;
        const oldRoot = tree.root;
        const existing = padToSize(tree.leaves.slice(0, chunkSize), chunkSize);

        const currentSiblings = proveChunk(tree, chunkIdx);

        // Insert 7 new leaves: 4 fit in current chunk, 3 overflow into chunk 1.
        const newLeaves = Array.from({ length: 7 }, () => randomBigInt());
        const paddedNew = padToSize(newLeaves, chunkSize);

        for (const l of newLeaves) tree.insert(l);
        const newRoot = tree.root;

        const nextSiblings = proveChunk(tree, chunkIdx + 1);

        await circuit.expectPass({
            oldRoot,
            newRoot,
            currentChunkFilled: BigInt(filled),
            currentChunkIndex: BigInt(chunkIdx),
            existingLeaves: existing,
            newLeaves: paddedNew,
            currentSiblings,
            nextSiblings,
        });
    });

    it("produces the same root as sequential SimpleIMT inserts", async () => {
        // Start with a tree that has 3 existing leaves.
        const tree = new SimpleIMT(D);
        for (let i = 0; i < 3; i++) tree.insert(randomBigInt());

        const filled = 3;
        const chunkIdx = 0;
        const oldRoot = tree.root;
        const existing = padToSize(tree.leaves.slice(0, chunkSize), chunkSize);

        const currentSiblings = proveChunk(tree, chunkIdx);

        // Insert 4 new leaves (no overflow).
        const newLeaves = Array.from({ length: 4 }, () => randomBigInt());
        const paddedNew = padToSize(newLeaves, chunkSize);

        for (const l of newLeaves) tree.insert(l);
        const newRoot = tree.root;

        const nextSiblings = proveChunk(tree, chunkIdx + 1);

        await circuit.expectPass({
            oldRoot,
            newRoot,
            currentChunkFilled: BigInt(filled),
            currentChunkIndex: BigInt(chunkIdx),
            existingLeaves: existing,
            newLeaves: paddedNew,
            currentSiblings,
            nextSiblings,
        });
    });
});
