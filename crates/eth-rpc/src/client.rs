use alloy_primitives::{Address, Bytes, FixedBytes, Log};
use alloy_sol_types::SolCall;
use common::MaybeSend;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tsify::Tsify;

#[cfg_attr(not(feature = "wasm"), async_trait::async_trait)]
#[cfg_attr(feature = "wasm", async_trait::async_trait(?Send))]
pub trait EthRpcClient: MaybeSend {
    async fn get_block_number(&self) -> Result<u64, EthRpcClientError>;

    async fn get_logs(
        &self,
        address: Address,
        event_signature: Option<FixedBytes<32>>,
        from_block: Option<u64>,
        to_block: Option<u64>,
    ) -> Result<Vec<RawLog>, EthRpcClientError>;

    async fn eth_call(&self, to: Address, data: Vec<u8>) -> Result<Vec<u8>, EthRpcClientError>;

    async fn estimate_gas(
        &self,
        to: Address,
        data: Vec<u8>,
        from: Option<Address>,
    ) -> Result<u64, EthRpcClientError>;

    async fn get_gas_price(&self) -> Result<u128, EthRpcClientError>;
}

#[derive(Debug, Error)]
pub enum EthRpcClientError {
    #[error("RPC error: {0}")]
    Rpc(String),
    #[error("Decode error: {0}")]
    Decode(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
pub struct RawLog {
    pub block_number: Option<u64>,
    pub block_timestamp: Option<u64>,
    #[tsify(type = "`0x${string}` | null")]
    pub transaction_hash: Option<FixedBytes<32>>,
    #[tsify(type = "`0x${string}`")]
    pub address: Address,
    #[tsify(type = "`0x${string}`[]")]
    pub topics: Vec<FixedBytes<32>>,
    pub data: Bytes,
}

impl RawLog {
    pub fn inner(&self) -> Log {
        Log::new_unchecked(self.address, self.topics.clone(), self.data.clone())
    }
}

pub async fn eth_call_sol<C>(
    provider: &dyn EthRpcClient,
    to: Address,
    call: C,
) -> Result<C::Return, EthRpcClientError>
where
    C: SolCall,
{
    let data = call.abi_encode();
    let raw = provider.eth_call(to, data).await?;
    C::abi_decode_returns(&raw).map_err(|e| EthRpcClientError::Decode(e.to_string()))
}
