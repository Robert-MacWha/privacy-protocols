#[cfg(not(feature = "wasm"))]
use alloy::sol;
#[cfg(feature = "wasm")]
use alloy_sol_types::sol;

#[cfg(not(feature = "wasm"))]
sol! {
    #[sol(rpc)]
    // ERC20 interface
    contract ERC20 {
        function approve(address spender, uint256 amount) external returns (bool);
        function allowance(address owner, address spender) external view returns (uint256);
        function balanceOf(address account) external view returns (uint256);
    }
}

#[cfg(feature = "wasm")]
sol! {
    // ERC20 interface
    contract ERC20 {
        function approve(address spender, uint256 amount) external returns (bool);
        function allowance(address owner, address spender) external view returns (uint256);
        function balanceOf(address account) external view returns (uint256);
    }
}
