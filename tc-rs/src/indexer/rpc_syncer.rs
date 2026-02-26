use alloy::{
    primitives::Address,
    providers::{DynProvider, Provider},
    rpc::types::Filter,
};
use alloy_sol_types::SolEvent;
use futures::{StreamExt, stream};
use ruint::aliases::U256;
use tracing::{info, warn};

use crate::{
    abis::tornado::{MerkleTreeWithHistory, Tornado},
    indexer::{
        syncer::{
            BoxedCommitmentStream, BoxedNullifierStream, Commitment, Nullifier, Syncer, SyncerError,
        },
        verifier::{Verifier, VerifierError},
    },
    merkle::MerkleRoot,
};

pub struct RpcSyncer {
    provider: DynProvider,
    contract_address: Address,
    batch_size: u64,
}

impl RpcSyncer {
    pub fn new(provider: DynProvider, contract_address: Address) -> Self {
        Self {
            provider,
            contract_address,
            batch_size: 2000,
        }
    }

    pub fn with_batch_size(mut self, batch_size: u64) -> Self {
        self.batch_size = batch_size;
        self
    }
}

#[cfg_attr(not(feature = "wasm"), async_trait::async_trait)]
#[cfg_attr(feature = "wasm", async_trait::async_trait(?Send))]
impl Syncer for RpcSyncer {
    async fn latest_block(&self) -> Result<u64, SyncerError> {
        self.provider
            .get_block_number()
            .await
            .map_err(|e| SyncerError::Syncer(Box::new(e)))
    }

    async fn sync_commitments(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<BoxedCommitmentStream<'_>, SyncerError> {
        let batch_size = self.batch_size;
        let contract_address = self.contract_address;
        let provider = &self.provider;

        let stream = stream::unfold(from_block, move |current_block| async move {
            if current_block > to_block {
                return None;
            }

            let batch_end = current_block
                .saturating_add(batch_size.saturating_sub(1))
                .min(to_block);
            let filter = Filter::new()
                .address(contract_address)
                .from_block(current_block)
                .to_block(batch_end);

            let logs = match provider.get_logs(&filter).await {
                Ok(logs) => logs,
                Err(e) => {
                    warn!(
                        "Failed to fetch logs for commitments {}-{}: {}",
                        current_block, batch_end, e
                    );
                    return None;
                }
            };

            let commitments: Vec<Commitment> = logs
                .into_iter()
                .filter_map(|log| {
                    if log.topics().first().copied() != Some(Tornado::Deposit::SIGNATURE_HASH) {
                        return None;
                    }
                    let block_number = log.block_number.unwrap_or(0);
                    let tx_hash = log.transaction_hash.unwrap_or_default();
                    match Tornado::Deposit::decode_log(&log.inner) {
                        Ok(event) => Some(Commitment {
                            block_number,
                            tx_hash,
                            commitment: event.data.commitment,
                            leaf_index: event.data.leafIndex,
                            timestamp: event.data.timestamp.saturating_to::<u64>(),
                        }),
                        Err(e) => {
                            warn!("Failed to decode Deposit event: {}", e);
                            None
                        }
                    }
                })
                .collect();

            info!(
                "Fetched {} commitments from blocks {}-{}",
                commitments.len(),
                current_block,
                batch_end
            );
            let next_block = batch_end + 1;
            Some((stream::iter(commitments), next_block))
        })
        .flatten();

        Ok(Box::pin(stream))
    }

    async fn sync_nullifiers(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<BoxedNullifierStream<'_>, SyncerError> {
        let batch_size = self.batch_size;
        let contract_address = self.contract_address;
        let provider = &self.provider;

        let stream = stream::unfold(from_block, move |current_block| async move {
            if current_block > to_block {
                return None;
            }

            let batch_end = current_block
                .saturating_add(batch_size.saturating_sub(1))
                .min(to_block);
            let filter = Filter::new()
                .address(contract_address)
                .from_block(current_block)
                .to_block(batch_end);

            let logs = match provider.get_logs(&filter).await {
                Ok(logs) => logs,
                Err(e) => {
                    warn!(
                        "Failed to fetch logs for nullifiers {}-{}: {}",
                        current_block, batch_end, e
                    );
                    return None;
                }
            };

            let nullifiers: Vec<Nullifier> = logs
                .into_iter()
                .filter_map(|log| {
                    if log.topics().first().copied() != Some(Tornado::Withdrawal::SIGNATURE_HASH) {
                        return None;
                    }
                    let block_number = log.block_number.unwrap_or(0);
                    let tx_hash = log.transaction_hash.unwrap_or_default();
                    match Tornado::Withdrawal::decode_log(&log.inner) {
                        Ok(event) => Some(Nullifier {
                            block_number,
                            tx_hash,
                            nullifier: event.data.nullifierHash,
                            to: event.data.to,
                            fee: event.data.fee.saturating_to::<u128>(),
                            timestamp: 0,
                        }),
                        Err(e) => {
                            warn!("Failed to decode Withdrawal event: {}", e);
                            None
                        }
                    }
                })
                .collect();

            info!(
                "Fetched {} nullifiers from blocks {}-{}",
                nullifiers.len(),
                current_block,
                batch_end
            );
            let next_block = batch_end + 1;
            Some((stream::iter(nullifiers), next_block))
        })
        .flatten();

        Ok(Box::pin(stream))
    }
}

#[cfg_attr(not(feature = "wasm"), async_trait::async_trait)]
#[cfg_attr(feature = "wasm", async_trait::async_trait(?Send))]
impl Verifier for RpcSyncer {
    async fn verify(&self, root: MerkleRoot) -> Result<(), VerifierError> {
        let contract = MerkleTreeWithHistory::new(self.contract_address, &self.provider);
        let root_b256 = alloy::primitives::FixedBytes::<32>::from(root);
        let result = contract
            .isKnownRoot(root_b256)
            .call()
            .await
            .map_err(|e| VerifierError::Other(Box::new(e)))?;

        let last = contract
            .getLastRoot()
            .call()
            .await
            .map_err(|e| VerifierError::Other(Box::new(e)))?;
        let last: U256 = last.into();
        info!("On-chain last root: {:?}", last);

        if result {
            Ok(())
        } else {
            Err(VerifierError::InvalidRoot { root })
        }
    }
}
