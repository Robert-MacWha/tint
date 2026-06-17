## Features
- UTXO-based
- Permissionless
- Programmable note spendability via spend circuits.
- Dual-trie design
  - Cheap-to-insert "Staging" trie for recent transactions.
  - Cheap-to-prove "Archive" trie for older transactions.  Deep 2-ary 32-depth poseidon tree.

### Programmable Spendability

Each note has a "spend circuit" that determines whether the note can be spent.  The spendability circuit can depend on arbitrary private inputs such as signatures, preimages, etc; and public inputs such as block number, timestamp, other notes in the same operation, etc. This enables a wide variety of use cases:
 - Multisig notes
 - Time-locked notes
 - Vesting notes
 - Order notes (private p2p swaps)

## Dual-Trie Design

In order to optimize for cheap insertion and cheap proof generation, a dual-trie design is used.  The smart contract maintains two merkle tries - a small "Staging" sha256 trie and a larger "Archive" poseidon trie. Deposits are first inserted into the Staging trie, which can be done cheaply on-chain. Once the Staging trie is full, its committments are migrated to the Archive trie in a single batch operation. 

Doing this means we can use a large, zk-efficient archive trie without incuring the gas cost of performing many poseidon hashes on-chain.  Users can also still spend notes from the Staging trie without waiting for migration, at the cost of more expensive proof generation.  Most users will likely wait for migration before spending to increase privacy and reduce proof generation time (and also just because it'll happen automatically).

## Architecture

### Circuits:
 - Spendability circuit: Determines whether a note can be spent.  This is a user-defined circuit that can depend on arbitrary private inputs and a limited set of public inputs.
 - Inclusion circuit: Two seperate inclusion circuits that prove the inclusion of a note in either the Staging or Archive trie.
 - Aggregator circuit: Aggregates the spendability and inclusion circuits for all notes in an operation into a single proof, enforcing constraints across the operation.
 - Graduation circuit: Proves the correct migration of a batch of notes from the Staging trie to the Archive trie.