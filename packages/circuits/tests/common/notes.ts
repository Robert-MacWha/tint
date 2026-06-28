import { poseidon2, poseidon3 } from "poseidon-lite";
import { LeanIMT } from "@zk-kit/lean-imt";
import { getFrontier } from ".";

export const N_INPUTS = 2;
export const N_OUTPUTS = 2;

const PRE_LEAVES = 1;
const DEPTH = 3;

const PRE_LEAF = 1337n;

const hash = (a: bigint, b: bigint) => poseidon2([a, b]);

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
    const tree = new LeanIMT(hash);
    tree.insert(PRE_LEAF);

    const initialFrontier = getFrontier(tree, DEPTH);
    const oldRoot = tree.root;
    const batchStartIndex = BigInt(PRE_LEAVES);

    for (const leaf of newLeaves) {
        tree.insert(leaf);
    }

    let aggHash = 0n;
    for (const leaf of newLeaves) {
        if (leaf !== 0n) aggHash = poseidon2([aggHash, leaf]);
    }

    // Input notes land at positions PRE_LEAVES+0, PRE_LEAVES+1 in the leanIMT.
    const leafIndicesIn = inputs.map((_, i) => batchStartIndex + BigInt(i));

    const nullifiers = inputs.map((n, i) =>
        n ? poseidon2([n.nullifyingKey, leafIndicesIn[i]]) : 0n
    );

    const siblingsIn = inputs.map((n, i) => {
        if (!n) return new Array(DEPTH).fill(0n);
        return tree.generateProof(PRE_LEAVES + i).siblings as bigint[];
    });

    return {
        oldRoot,
        newRoot: tree.root,
        leavesAggregationHash: aggHash,
        nullifiers,
        commitmentsOut,
        unshieldAmounts: Array(N_OUTPUTS).fill(0n),
        unshieldAssets: Array(N_OUTPUTS).fill(0n),
        boundParamsHash: 1n,

        batchStartIndex,
        newLeaves,
        initialFrontier,
        startingAggregationHash: 0n,

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
