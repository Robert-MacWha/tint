# Note encryption

Note ciphertexts live in event data forever, so anything that encrypts them must survive a future quantum computer ("harvest now, decrypt later"). Tint encrypts notes with **X-Wing** (ML-KEM-768 + X25519), using **one encapsulation per operation**, plus a one-byte classical **view tag** so scanners skip most operations without running ML-KEM.

## Scheme

**Keys.** An account's 32-byte encryption seed (HKDF of the account seed) is an X-Wing decapsulation key. From it the account also derives:

- a view key: X-Wing's own X25519 secret, used alone to check view tags;
- a self-secret: a symmetric key the account seals notes to itself under.

A receiving address (`PubAccount`) is the nullifier public key (32 B), the X-Wing encapsulation key (1,216 B) and the policy hash (32 B): **1,280 B**, up from 96 B.

**Header.** `viewTag (1 B) || X-Wing ciphertext (1,120 B)`, where `viewTag` is the first byte of `sha256("tint/view-tag/v1" || ss_x)` and `ss_x` is the X25519 shared secret of X-Wing's own X25519 component (the ephemeral key in its ciphertext, the X25519 key in its encapsulation key). No extra key or ephemeral is sent for it.

**Note ciphertexts.** Each note is sealed with ChaCha20-Poly1305 under `hkdf(secret, "tint/note-key/v1" || commitment)` with a zero nonce (152 B for a full note).

**Operations.** The first output's ciphertext is `header || sealed note`; the others are just a sealed note:

- Outputs back to the sender (change) are sealed under the sender's self-secret.
- Outputs to the receiver are sealed under the secret the header encapsulates to them.
- An operation may pay **one receiver besides the sender** (`OperationError::TooManyReceivers` otherwise). If it pays nobody else, the header encapsulates to the sender.

So every operation with `n` outputs emits exactly `1,121 + 152n` bytes of ciphertext, with the header always on output 0.

**Deposits.** `header || sealed partial note` (1,236 B). A paymaster refund deposit encrypts its partial before the refund amount, and so its commitment, is known, so partials are sealed for commitment `0`; the deposit's encapsulated secret is fresh and seals nothing else.

## Gas

Real transaction gas from anvil receipts (`packages/contracts/script/NoteEncryptionGas.s.sol`, anvil 1.8.4 with Prague rules): every case is sent from the same `evm_snapshot` of a pool holding 6 notes, against the real Groth16 pairing check, with random ciphertext bytes of each layout's length. "MxN" is inputs x outputs.

| Case | No ciphertext | X25519 (old) | X-Wing per note | ML-KEM-512+X25519 per note | **X-Wing per op** | ML-KEM-512+X25519 per op |
| --- | --- | --- | --- | --- | --- | --- |
| Deposit | 121,612 | 127,007 | 151,623 | 144,680 | **151,623** | 144,680 |
| 1x1 | 367,623 | 374,394 | 399,041 | 391,941 | **399,029** | 391,941 |
| 1x2 (transfer + change) | 380,222 | 393,769 | 442,898 | 428,789 | **415,535** | 408,387 |
| 5x5 | 511,318 | 545,039 | 667,568 | 632,356 | **558,025** | 550,996 |

Ciphertext bytes, for `n` outputs:

| Layout | Formula | Deposit | 1x1 | 1x2 | 5x5 |
| --- | --- | --- | --- | --- | --- |
| X25519 (old, postcard envelope) | 260n | 223 | 260 | 520 | 1,300 |
| X-Wing per note | (1,121 + 152)n | 1,236 | 1,273 | 2,546 | 6,365 |
| ML-KEM-512+X25519 per note | (833 + 152)n | 948 | 985 | 1,970 | 4,925 |
| **X-Wing per op** | 1,121 + 152n | 1,236 | 1,273 | 1,425 | 1,881 |
| ML-KEM-512+X25519 per op | 833 + 152n | 948 | 985 | 1,137 | 1,593 |

Each ciphertext byte costs about **24.4 gas**: 16 for non-zero calldata, 8 for LOG data, the rest copying it into memory and hashing it into `boundParamsHash`. The EIP-7623 calldata floor (10/40 gas per zero/non-zero byte) never binds: proof verification dominates, e.g. the 5x5 per-note floor is ~300k gas against 668k used. On rollups, where data dominates the fee, the gaps between layouts are wider.

Per-op X-Wing costs a 2-output transfer 21.8k gas more than the old X25519 scheme, and 27.4k less than per-note X-Wing (109.5k less at 5x5). Deposits cost 24.6k more than before.

## Discovery

Measured per header or output in a release build, on a machine busy with other builds (best of three runs; treat as upper bounds, the ratios are what matter):

| Step | Time |
| --- | --- |
| View-tag check, rejecting (X25519 + SHA-256) | ~61 µs |
| View-tag check + X-Wing decapsulation, matching | ~320 µs |
| Self-secret trial on an output, failing (HKDF + AEAD) | ~39 µs |

A scanner does one view-tag check per operation and deposit, one self-secret trial per output, and a full decapsulation only for headers addressed to it plus ~1/256 of everyone else's (10 false matches in 2,000 in the run above). Without view tags every header would need a full decapsulation, about 5x the cost.

## Candidates

- **X-Wing per note.** Simplest; 1,121 B per output. Rejected for gas: a transfer pays for two encapsulations.
- **ML-KEM-512 hybrids.** 288 B smaller per encapsulation, about 7k gas per operation or deposit once amortized (~1.7% of a transfer). Rejected: no recognised hybrid pairs ML-KEM-512 (X-Wing, TLS's `X25519MLKEM768` and the CFRG/HPKE hybrids all use ML-KEM-768 or -1024), and it's NIST's lowest security category. The saving doesn't justify a non-standard construction.
- **Pure ML-KEM-768.** 32 B (~800 gas) smaller than X-Wing. Rejected: the hybrid hedges against ML-KEM cryptanalysis or implementation bugs for almost nothing.
- **One header per distinct receiver.** Supports paying several parties in one operation, but the header count reveals how many distinct receivers an operation has, and header positions reveal which outputs share one. Rejected for a fixed one header per operation.
- **Reusing a KEM secret across operations** (sender-side caching, or a first-contact handshake followed by per-payment tags). Diffie-Hellman lets two static keys agree on a secret with nothing sent; a KEM can't, so every secret has to be delivered as a ciphertext at least once. Reusing one needs both parties to keep per-counterparty state (which seed-only recovery then depends on), and first-contact operations would be 1,120 B longer than repeat payments, revealing first contact. Rejected for now.
- **Shorter note plaintext.** The policy hash and nullifier public key (64 B, ~1.6k gas per output) are the receiver's own and could be left out. Orthogonal to the KEM choice; not done here.

## Trade-offs

- **One receiver per operation.** Paying several parties takes one operation each. This is the cost of hiding how many parties an operation pays.
- **Ciphertext lengths.** They depend only on the output count, which is already public; deposits are all the same length. Whether an operation pays someone else or only its sender is hidden.
- **View tags and quantum adversaries.** The tag is 8 bits derived from the X25519 secret. Classically it reveals nothing and costs X-Wing at most 8 bits of its classical security, like ERC-5564 view tags. An adversary who can break X25519 and knows an address can use tags to discard all but ~1/256 of operations as not paying that address: a weak linkability leak. They still can't decrypt anything, as ML-KEM protects the contents.
- **Address size.** 1,280 B (2,560 hex characters) is too long to paste comfortably. It fits a single QR code in binary mode; for typed or spoken sharing, publish it somewhere (e.g. an ENS text record or a registry contract) and share a short name.

## Security

- **X-Wing** (`draft-connolly-cfrg-xwing-kem`, RustCrypto `x-wing` 0.1.1 implementing draft-06) is IND-CCA secure if either ML-KEM-768 (FIPS 203) or X25519 is. The view tag reproduces X-Wing's key expansion to reuse its X25519 key; a unit test checks it against the crate.
- **ChaCha20-Poly1305** with 256-bit keys keeps 128-bit security against Grover. Each key seals one message, so the fixed nonce is safe: note keys are derived per commitment, and commitments are unique (32 fresh random bytes); a deposit's secret seals only its partial.
- **Self-secret** encryption is purely symmetric, so it's post-quantum as is.
- **Commitment check.** A decrypted note must still hash to its on-chain commitment (the AEAD binding stops ciphertexts from being moved between commitments, but a sender chooses the plaintext).
