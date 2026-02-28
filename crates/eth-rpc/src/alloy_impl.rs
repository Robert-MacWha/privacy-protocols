use alloy::{
    network::TransactionBuilder,
    providers::Provider,
    rpc::types::{Filter, TransactionRequest},
    transports::{RpcError, TransportErrorKind},
};
use alloy_primitives::{Address, FixedBytes};

use crate::{EthRpcClient, EthRpcClientError, RawLog};

#[async_trait::async_trait]
impl<P: Provider> EthRpcClient for P {
    async fn get_block_number(&self) -> Result<u64, EthRpcClientError> {
        Ok(self.get_block_number().await?)
    }

    async fn get_logs(
        &self,
        address: Address,
        event_signature: FixedBytes<32>,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<RawLog>, EthRpcClientError> {
        let filter = Filter::new()
            .address(address)
            .event_signature(event_signature)
            .from_block(from_block)
            .to_block(to_block);

        let logs = self.get_logs(&filter).await?;
        let logs = logs
            .into_iter()
            .map(|log| RawLog {
                address: log.address(),
                topics: log.topics().to_vec(),
                data: log.data().clone(),
                block_number: log.block_number,
                block_timestamp: log.block_timestamp,
                transaction_hash: log.transaction_hash,
            })
            .collect();

        Ok(logs)
    }

    async fn eth_call(&self, to: Address, data: Vec<u8>) -> Result<Vec<u8>, EthRpcClientError> {
        let request = TransactionRequest::default().to(to).with_input(data);
        Ok(self.call(request).await?.into())
    }

    async fn estimate_gas(
        &self,
        to: Address,
        data: Vec<u8>,
        from: Option<Address>,
    ) -> Result<u64, EthRpcClientError> {
        let mut request = TransactionRequest::default().to(to).with_input(data);
        if let Some(f) = from {
            request = request.from(f);
        }
        Ok(self.estimate_gas(request).await?)
    }

    async fn get_gas_price(&self) -> Result<u128, EthRpcClientError> {
        Ok(self.get_gas_price().await?)
    }
}

impl From<RpcError<TransportErrorKind>> for EthRpcClientError {
    fn from(e: RpcError<TransportErrorKind>) -> Self {
        EthRpcClientError::Rpc(e.to_string())
    }
}
