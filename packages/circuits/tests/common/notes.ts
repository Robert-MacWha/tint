import { poseidon2, poseidon3, poseidon4 } from "poseidon-lite";

export const N_INPUTS = 2;
export const N_OUTPUTS = 2;
export const INNER_DEPTH = 1;
export const OUTER_DEPTH = 1;

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

// Build the full aggregator witness for a 2-in / 2-out circuit (innerDepth=1, outerDepth=1).
export function buildAggregatorInput(
    inputs: (Note | null)[],
    outputs: Output[],
    batchIndex = 0
) {
    const commitmentsIn = inputs.map(n =>
        n ? poseidon3([n.asset, n.amount, n.partialCommitment]) : 0n
    );
    const commitmentsOut = outputs.map(o =>
        poseidon3([o.asset, o.amount, o.partialCommitment])
    );
    const newLeaves = [...commitmentsIn, ...commitmentsOut];

    // Outer tree has depth 1: root = poseidon4([slot0, slot1, slot2, slot3]).
    // Old root has all slots empty; new root inserts batchRoot at batchIndex.
    const batchRoot = poseidon4(newLeaves);
    const outerSlots = [0n, 0n, 0n, 0n];
    outerSlots[batchIndex] = batchRoot;
    const oldRoot = poseidon4([0n, 0n, 0n, 0n]);
    const newRoot = poseidon4(outerSlots);

    // leavesAggregationHash: sequential poseidon2 accumulation over non-zero leaves.
    let aggHash = 0n;
    for (const leaf of newLeaves) {
        if (leaf !== 0n) aggHash = poseidon2([aggHash, leaf]);
    }

    // Per-input: leafIndex = batchIndex * 4 + batchPosition.
    const leafIndicesIn = inputs.map((_, i) => BigInt(batchIndex * 4 + i));

    const nullifiers = inputs.map((n, i) =>
        n ? poseidon2([n.nullifyingKey, leafIndicesIn[i]]) : 0n
    );

    // Merkle proof for each input:
    //   level 0 (inner): the 3 other leaves in the batch
    //   level 1 (outer): [0,0,0] — the other 3 outer slots are always empty
    const siblingsIn = inputs.map((_, i) => [
        [0, 1, 2, 3].filter(k => k !== i).map(k => newLeaves[k]),
        [0n, 0n, 0n],
    ]);

    return {
        oldRoot,
        newRoot,
        leavesAggregationHash: aggHash,
        nullifiers,
        commitmentsOut,
        unshieldAmounts: [0n, 0n],
        unshieldAssets: [0n, 0n],
        boundParamsHash: 1n,

        newLeaves,
        startingAggregationHash: 0n,
        batchIndex: BigInt(batchIndex),
        batchSiblings: [[0n, 0n, 0n]],

        commitmentsIn,
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
