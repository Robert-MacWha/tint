import { Circomkit } from "circomkit";
import { poseidon2 } from "poseidon-lite";

export const circomkit = new Circomkit({
  verbose: false,
});

export function randomBigInt(): bigint {
  return BigInt(Math.floor(Math.random() * Number.MAX_SAFE_INTEGER));
}

/// Standard fixed-depth incremental Merkle tree reference implementation.
export class SimpleIMT {
    private readonly depth: number;
    private readonly zeros: bigint[];
    private _frontier: bigint[];
    private _leaves: bigint[];
    private _root: bigint;

    constructor(depth: number) {
        this.depth = depth;
        this.zeros = this.computeZeros();
        this._frontier = new Array(depth).fill(0n);
        this._leaves = [];
        this._root = this.zeros[depth];
    }

    private computeZeros(): bigint[] {
        const zeros = [0n];
        for (let i = 1; i <= this.depth; i++) {
            zeros.push(poseidon2([zeros[i - 1], zeros[i - 1]]));
        }
        return zeros;
    }

    insert(leaf: bigint): void {
        const index = this._leaves.length;
        let current = leaf;

        for (let l = 0; l < this.depth; l++) {
            if (((index >> l) & 1) === 0) {
                this._frontier[l] = current;
                current = poseidon2([current, this.zeros[l]]);
            } else {
                current = poseidon2([this._frontier[l], current]);
            }
        }

        this._root = current;
        this._leaves.push(leaf);
    }

    get root(): bigint { return this._root; }
    get size(): number { return this._leaves.length; }
    get frontier(): bigint[] { return [...this._frontier]; }
    get leaves(): bigint[] { return [...this._leaves]; }

    /// Returns sibling path from leaf at `index` to the root (depth entries).
    generateProof(index: number): bigint[] {
        // Compute nodes on demand using a sparse recursive tree.
        // Empty leaf positions use 0n, which propagates correctly through Poseidon.
        const cache = new Map<string, bigint>();

        const getNode = (level: number, pos: number): bigint => {
            if (level === 0) {
                return pos < this._leaves.length ? this._leaves[pos] : 0n;
            }
            const key = `${level},${pos}`;
            if (cache.has(key)) return cache.get(key)!;
            const val = poseidon2([getNode(level - 1, pos * 2), getNode(level - 1, pos * 2 + 1)]);
            cache.set(key, val);
            return val;
        };

        const siblings: bigint[] = [];
        let idx = index;
        for (let l = 0; l < this.depth; l++) {
            siblings.push(getNode(l, idx ^ 1));
            idx >>= 1;
        }
        return siblings;
    }
}
