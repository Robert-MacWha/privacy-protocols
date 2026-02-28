mod client;

#[cfg(feature = "native")]
pub mod alloy_impl;

#[cfg(feature = "wasm")]
pub mod wasm_impl;

pub use client::{EthRpcClient, EthRpcClientError, RawLog, eth_call_sol};
#[cfg(feature = "wasm")]
pub use wasm_impl::JsEthRpcAdapter;
