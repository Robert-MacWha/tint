import { poseidon1, poseidon2, poseidon3 } from "poseidon-lite";
import { SimpleIMT } from ".";

export const N_INPUTS = 2;
export const N_OUTPUTS = 2;

export const CHUNK_DEPTH = 2;           // chunkSize = 2^CHUNK_DEPTH = 4
export const CHUNK_SIZE = 1 << CHUNK_DEPTH;
export const DEPTH = 3;
export const CHUNK_PATH_LEN = DEPTH - CHUNK_DEPTH; // = 1

const PRE_LEAF = 1337n;
const PRE_LEAVES = 1; // one leaf in the tree before the batch

export type Note = {
    asset: bigint;
    amount: bigint;
    nullifyingKey: bigint;    // private key — circuit derives pubKey in-circuit
    spendabilityHash: bigint; // keccak(address, data); use any bigint in tests
    random: bigint;           // uniqueness
};

export type Output = {
    asset: bigint;
    amount: bigint;
    nullifyingPubKey: bigint; // poseidon1([nullifyingKey])
    spendabilityHash: bigint;
    random: bigint;
};

function nullPubKey(privKey: bigint): bigint {
    return poseidon1([privKey]);
}

function partialCommit(sh: bigint, pubKey: bigint, rand: bigint): bigint {
    return poseidon3([sh, pubKey, rand]);
}

function commit(asset: bigint, amount: bigint, partial: bigint): bigint {
    return poseidon3([asset, amount, partial]);
}

function noteCommitment(n: Note): bigint {
    const pub = nullPubKey(n.nullifyingKey);
    return commit(n.asset, n.amount, partialCommit(n.spendabilityHash, pub, n.random));
}

function outputCommitment(o: Output): bigint {
    return commit(o.asset, o.amount, partialCommit(o.spendabilityHash, o.nullifyingPubKey, o.random));
}

export function buildAggregatorInput(
    inputs: (Note | null)[],
    outputs: Output[],
) {
    // Compute all commitments.
    const commitmentsIn = inputs.map(n => n ? noteCommitment(n) : 0n);
    const commitmentsOut = outputs.map(o => outputCommitment(o));

    // New leaves = input commitments (so they land in the tree) + output commitments.
    // Zero-padded to CHUNK_SIZE.
    const activeLeaves = [...commitmentsIn, ...commitmentsOut];
    const newLeaves = [...activeLeaves, ...new Array(CHUNK_SIZE - activeLeaves.length).fill(0n)];

    // Build the tree pre-insert.
    const tree = new SimpleIMT(DEPTH);
    tree.insert(PRE_LEAF);

    const oldRoot = tree.root;
    const chunkIdx = 0;
    const filled = PRE_LEAVES; // 1 leaf already in chunk 0
    const existingChunkLeaves = new Array(CHUNK_SIZE).fill(0n);
    existingChunkLeaves[0] = PRE_LEAF;

    // currentSiblings: siblings of chunk 0 in the OLD tree (before inserting new leaves).
    const currentSiblings = proveChunk(tree, chunkIdx);

    // Insert all new leaves into the tree.
    for (const leaf of newLeaves) {
        tree.insert(leaf);
    }
    const newRoot = tree.root;

    // nextSiblings: siblings of chunk 1 in the POST-INSERT tree (= intermediate tree for chunk 0 update).
    const nextSiblings = proveChunk(tree, chunkIdx + 1);

    // Aggregation hash: sequential Poseidon accumulation over non-zero new leaves.
    let endAggregationHash = 0n;
    for (const leaf of newLeaves) {
        if (leaf !== 0n) endAggregationHash = poseidon2([endAggregationHash, leaf]);
    }

    // Input notes: positions in the tree after batch insert.
    // newLeaves[0] → tree[PRE_LEAVES + 0], newLeaves[1] → tree[PRE_LEAVES + 1], etc.
    const leafIndicesIn = inputs.map((_, i) => BigInt(PRE_LEAVES + i));

    const nullifiers = inputs.map((n, i) =>
        n ? poseidon2([n.nullifyingKey, leafIndicesIn[i]]) : 0n
    );

    const siblingsIn = inputs.map((n, i) => {
        if (!n) return new Array(DEPTH).fill(0n);
        return tree.generateProof(PRE_LEAVES + i);
    });

    return {
        // Public
        oldRoot,
        newRoot,
        startAggregationHash: 0n,
        endAggregationHash,
        nullifiers,
        commitmentsOut,
        unshieldAmounts: Array(N_OUTPUTS).fill(0n),
        unshieldAssets: Array(N_OUTPUTS).fill(0n),
        boundParamsHash: 1n,
        spendabilityHashesIn: inputs.map(n => n?.spendabilityHash ?? 0n),
        spendabilityHashesOut: outputs.map(o => o.spendabilityHash),

        // Private — chunk insert
        newLeaves,
        currentChunkFilled: BigInt(filled),
        currentChunkIndex: BigInt(chunkIdx),
        existingChunkLeaves,
        currentChunkSiblings: currentSiblings,
        nextChunkSiblings: nextSiblings,

        // Private — input notes
        siblingsIn,
        leafIndicesIn,
        assetsIn: inputs.map(n => n?.asset ?? 0n),
        amountsIn: inputs.map(n => n?.amount ?? 0n),
        nullifyingKeysIn: inputs.map(n => n?.nullifyingKey ?? 0n),
        randomIn: inputs.map(n => n?.random ?? 0n),

        // Private — output notes
        assetsOut: outputs.map(o => o.asset),
        amountsOut: outputs.map(o => o.amount),
        nullifyingPubKeysOut: outputs.map(o => o.nullifyingPubKey),
        randomOut: outputs.map(o => o.random),
    };
}

function proveChunk(tree: SimpleIMT, chunkIndex: number): bigint[] {
    const leafIndex = chunkIndex * CHUNK_SIZE;
    const fullProof = tree.generateProof(leafIndex);
    return fullProof.slice(CHUNK_DEPTH);
}
