import { Circomkit } from "circomkit";
import { LeanIMT } from "@zk-kit/lean-imt";
import { poseidon2 } from "poseidon-lite";

export const circomkit = new Circomkit({
  verbose: false,
});

export function randomBigInt(): bigint {
  return BigInt(Math.floor(Math.random() * Number.MAX_SAFE_INTEGER));
}

export function getFrontier(tree: LeanIMT, maxDepth: number): bigint[] {
    const frontier: bigint[] = new Array(maxDepth).fill(0n);
    for (let i = 0; i < tree.size; i++) {
        let node = (tree.leaves as bigint[])[i];
        for (let l = 0; l < maxDepth; l++) {
            if (((i >> l) & 1) === 0) {
                frontier[l] = node;
                break;
            } else {
                node = poseidon2([frontier[l], node]);
            }
        }
    }
    return frontier;
}