- [x] Implement deferred spending in the contracts
- [x] Implement spendability circuits
- [ ] Implement paymaster contract

- [x] Switch to `create_proof_with_reduction_and_matrices`
- [ ] Add proper tests for indexer + provider

- [ ] Reduce gas cost by using sha256 for commitment hash instead of poseidon2
- [ ] Reduce gas cost by merging asset/amount into single field element.
  - A single bn254 can hold ~253 bits, so can store address (160 bits) + amount (93 bits).
  - 93 bits is only ~9e+27. Most erc20s use 18 decimals, so this is equivalent to 9e+9 tokens. Sufficient for most use cases but some memcoins and dao tokens may exceed this. For example, 9e+9 PEPE is just $26,100 USD, which someone could conceivably exceed.
- [ ] Reduce gas cost by using sha256 to compress public inputs
