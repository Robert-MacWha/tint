use alloy_sol_macro::sol;

sol! {
    // ERC20 interface
    contract ERC20 {
        function approve(address spender, uint256 amount) external returns (bool);
    }
}
