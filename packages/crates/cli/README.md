# Tint Toybox CLI

A very simple toybox CLI that demonstrates tint's features:
- [x] ERC20 shields
- [x] ERC20 internal transfers
- [x] ERC20 unshields
- [ ] Automatic multi-input note selection
- [ ] 4337-relayed transfers and unshields
- [ ] Paymaster support
- [ ] Custom spendability policies

## Usage

Run `just run help` or `cargo run --release -- help` to see the available commands.

### Example

```bash
export TOKEN=0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2

# Fund EOA with 100 ETH and WETH
just run set-balance 100000000000000000000
just run set-erc20-balance $TOKEN 100000000000000000000

# Create tint accounts
just run create-account alice
just run create-account bob

# Shield into alice's account
just run shield alice $TOKEN 1000
just run transfer alice bob $TOKEN 500
just run unshield alice 0x000000000000000000000000000000000000dead $TOKEN 400

just run balance alice
just run balance bob
```
