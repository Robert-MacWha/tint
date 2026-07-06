import { poseidon2, poseidon3 } from "poseidon-lite";
import { SimpleIMT } from ".";

export const N_INPUTS = 2;
export const N_OUTPUTS = 2;

const PRE_LEAVES = 1;
const DEPTH = 3;

const PRE_LEAF = 1337n;

export type Note = {
    asset: bigint;
    amount: bigint;
    nullifyingKey: bigint;
    partialCommitment: bigint;
};

export type Output = {
    asset: bigint;
    amount: bigint;
    partialCommitment: bigint;
};

export function buildAggregatorInput(
    inputs: (Note | null)[],
    outputs: Output[],
) {
    const commitmentsIn = inputs.map(n =>
        n ? poseidon3([n.asset, n.amount, n.partialCommitment]) : 0n
    );
    const commitmentsOut = outputs.map(o =>
        poseidon3([o.asset, o.amount, o.partialCommitment])
    );

    const newLeaves = [...commitmentsIn, ...commitmentsOut];

    // Build tree: 1 pre-existing leaf, then the batch.
    const tree = new SimpleIMT(DEPTH);
    tree.insert(PRE_LEAF);

    const initialFrontier = tree.frontier;
    const oldRoot = tree.root;
    const batchStartIndex = BigInt(PRE_LEAVES);

    for (const leaf of newLeaves) {
        tree.insert(leaf);
    }

    let endAggregationHash = 0n;
    for (const leaf of newLeaves) {
        if (leaf !== 0n) endAggregationHash = poseidon2([endAggregationHash, leaf]);
    }

    // Input notes land at positions PRE_LEAVES+0, PRE_LEAVES+1 in the tree.
    const leafIndicesIn = inputs.map((_, i) => batchStartIndex + BigInt(i));

    const nullifiers = inputs.map((n, i) =>
        n ? poseidon2([n.nullifyingKey, leafIndicesIn[i]]) : 0n
    );

    const siblingsIn = inputs.map((n, i) => {
        if (!n) return new Array(DEPTH).fill(0n);
        return tree.generateProof(PRE_LEAVES + i);
    });

    return {
        oldRoot,
        newRoot: tree.root,
        startAggregationHash: 0n,
        endAggregationHash,
        nullifiers,
        commitmentsOut,
        unshieldAmounts: Array(N_OUTPUTS).fill(0n),
        unshieldAssets: Array(N_OUTPUTS).fill(0n),
        boundParamsHash: 1n,

        batchStartIndex,
        newLeaves,
        initialFrontier,

        siblingsIn,
        leafIndicesIn,
        assetsIn: inputs.map(n => n?.asset ?? 0n),
        amountsIn: inputs.map(n => n?.amount ?? 0n),
        nullifyingKeysIn: inputs.map(n => n?.nullifyingKey ?? 0n),
        partialCommitmentsIn: inputs.map(n => n?.partialCommitment ?? 0n),

        assetsOut: outputs.map(o => o.asset),
        amountsOut: outputs.map(o => o.amount),
        partialCommitmentsOut: outputs.map(o => o.partialCommitment),
    };
}
