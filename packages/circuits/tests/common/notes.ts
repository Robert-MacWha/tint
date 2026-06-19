import { poseidon1, poseidon2, poseidon3, poseidon4 } from "poseidon-lite";

// Circuit parameters, kept in sync with circuits.json / main/aggregator.circom.
export const STAGING_DEPTH = 8;
export const ARCHIVE_DEPTH = 40;

/// An input note. Mirrors the private signals consumed per input by the circuit.
export type Note = {
  asset: bigint;
  amount: bigint;
  random: bigint;
  nullifyingKey: bigint;
  spendabilityAddress: bigint;
  spendabilityData: bigint;
};

/// An output note. The sender only knows the partial commitment, so we mirror
/// that: the circuit takes the partial commitment directly and never re-derives it.
export type Output = {
  asset: bigint;
  amount: bigint;
  partial: bigint;
};

export function partialCommitment(n: Note): bigint {
  const nullifyingPubKey = poseidon1([n.nullifyingKey]);
  return poseidon4([n.random, nullifyingPubKey, n.spendabilityAddress, n.spendabilityData]);
}

export function commitment(n: Note): bigint {
  return poseidon3([n.asset, n.amount, partialCommitment(n)]);
}

export function outputCommitment(o: Output): bigint {
  return poseidon3([o.asset, o.amount, o.partial]);
}

export function nullifier(n: Note): bigint {
  return poseidon2([n.nullifyingKey, commitment(n)]);
}

/// Minimal LeanIMT that reproduces the exact rule the circuit verifies:
///   - a node with two children is poseidon2(left, right)
///   - a node with a single (left) child propagates that child's value upward
/// Proofs are emitted as a leaf index plus a zero-padded sibling list, matching
/// LeanIMTProofVerifier (which treats a zero sibling as "empty / propagate").
export class LeanIMT {
  readonly levels: bigint[][];

  constructor(leaves: bigint[]) {
    if (leaves.length === 0) throw new Error("LeanIMT requires at least one leaf");
    this.levels = [leaves.slice()];
    let current = leaves.slice();
    while (current.length > 1) {
      const next: bigint[] = [];
      for (let i = 0; i < current.length; i += 2) {
        if (i + 1 < current.length) {
          next.push(poseidon2([current[i], current[i + 1]]));
        } else {
          next.push(current[i]);
        }
      }
      this.levels.push(next);
      current = next;
    }
  }

  get root(): bigint {
    return this.levels[this.levels.length - 1][0];
  }

  get depth(): number {
    return this.levels.length - 1;
  }

  proof(index: number, maxDepth: number): { leafIndex: bigint; siblings: bigint[] } {
    const siblings: bigint[] = [];
    let cursor = index;
    for (let level = 0; level < this.depth; level++) {
      const isRight = cursor & 1;
      const siblingIndex = isRight ? cursor - 1 : cursor + 1;
      const nodes = this.levels[level];
      siblings.push(siblingIndex < nodes.length ? nodes[siblingIndex] : 0n);
      cursor >>= 1;
    }
    while (siblings.length < maxDepth) siblings.push(0n);
    if (siblings.length !== maxDepth) throw new Error("proof deeper than maxDepth");
    return { leafIndex: BigInt(index), siblings };
  }
}

/// A "missing" proof: all-zero siblings make the verifier propagate the leaf
/// unchanged, so the computed root equals the leaf (the note's commitment).
/// As long as that commitment is not the other tree's root, the OR-check passes
/// on the branch where the note actually lives.
function emptyProof(maxDepth: number): { leafIndex: bigint; siblings: bigint[] } {
  return { leafIndex: 0n, siblings: new Array(maxDepth).fill(0n) };
}

/// Distinct, non-zero filler leaf so trees never contain a zero node (which the
/// verifier would interpret as "empty").
function filler(i: number): bigint {
  return poseidon1([BigInt(900_000 + i)]);
}

function buildTree(entries: { index: number; leaf: bigint }[]): LeanIMT {
  const maxIndex = Math.max(...entries.map(e => e.index));
  const leaves: bigint[] = [];
  for (let i = 0; i <= maxIndex; i++) leaves.push(filler(i));
  for (const e of entries) leaves[e.index] = e.leaf;
  return new LeanIMT(leaves);
}

/// Which tree an input note lives in, and at what index.
export type InputPlacement = Note & { tree: "staging" | "archive"; index: number };

/// Assembles a complete, witness-ready input object for the Aggregator circuit.
/// Each input gets a real proof for its resident tree and an empty proof for the
/// other. Callers can mutate the returned object to craft negative test cases.
export function buildAggregatorInput(inputs: InputPlacement[], outputs: Output[]) {
  const stagingEntries = inputs
    .filter(i => i.tree === "staging")
    .map(i => ({ index: i.index, leaf: commitment(i) }));
  const archiveEntries = inputs
    .filter(i => i.tree === "archive")
    .map(i => ({ index: i.index, leaf: commitment(i) }));

  // Each tree always needs at least one leaf so it has a well-defined root, even
  // if no input happens to live there.
  const stagingTree = buildTree(stagingEntries.length ? stagingEntries : [{ index: 0, leaf: filler(0) }]);
  const archiveTree = buildTree(archiveEntries.length ? archiveEntries : [{ index: 0, leaf: filler(0) }]);

  const proofs = inputs.map(input => {
    if (input.tree === "staging") {
      return {
        staging: stagingTree.proof(input.index, STAGING_DEPTH),
        archive: emptyProof(ARCHIVE_DEPTH),
      };
    }
    return {
      staging: emptyProof(STAGING_DEPTH),
      archive: archiveTree.proof(input.index, ARCHIVE_DEPTH),
    };
  });

  return {
    stagingRoot: stagingTree.root,
    archiveRoot: archiveTree.root,
    nullifiers: inputs.map(nullifier),
    commitmentsOut: outputs.map(outputCommitment),
    unshieldRecipients: outputs.map(() => 0n),
    unshieldAmounts: outputs.map(() => 0n),
    unshieldAssets: outputs.map(() => 0n),

    stagingPathElementsIn: proofs.map(p => p.staging.siblings),
    archivePathElementsIn: proofs.map(p => p.archive.siblings),
    stagingLeafIndicesIn: proofs.map(p => p.staging.leafIndex),
    archiveLeafIndicesIn: proofs.map(p => p.archive.leafIndex),

    assetsIn: inputs.map(i => i.asset),
    amountsIn: inputs.map(i => i.amount),
    randomsIn: inputs.map(i => i.random),
    nullifyingKeysIn: inputs.map(i => i.nullifyingKey),
    spendabilityAddressesIn: inputs.map(i => i.spendabilityAddress),
    spendabilityDataIn: inputs.map(i => i.spendabilityData),

    assetsOut: outputs.map(o => o.asset),
    amountsOut: outputs.map(o => o.amount),
    partialCommitmentsOut: outputs.map(o => o.partial),
  };
}
