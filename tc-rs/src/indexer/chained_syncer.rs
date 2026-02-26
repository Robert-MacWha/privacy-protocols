use std::sync::Arc;

use alloy::primitives::Address;
use futures::{StreamExt, stream};

use crate::{
    indexer::{
        Syncer, SyncerError,
        syncer::{BoxedCommitmentStream, BoxedNullifierStream},
    },
};

/// A syncer that chains multiple syncers in priority order
pub struct ChainedSyncer {
    syncers: Vec<Arc<dyn Syncer>>,
}

impl ChainedSyncer {
    /// Creates a new ChainedSyncer with the given syncers in priority order
    ///
    /// Syncers will be queried in the order they are provided, first to last
    pub fn new(syncers: Vec<Arc<dyn Syncer>>) -> Self {
        Self { syncers }
    }
}

#[cfg_attr(not(feature = "wasm"), async_trait::async_trait)]
#[cfg_attr(feature = "wasm", async_trait::async_trait(?Send))]
impl Syncer for ChainedSyncer {
    async fn latest_block(&self) -> Result<u64, SyncerError> {
        let mut max_block = 0u64;
        for syncer in &self.syncers {
            if let Ok(block) = syncer.latest_block().await {
                max_block = max_block.max(block);
            }
        }
        Ok(max_block)
    }

    async fn sync_commitments(
        &self,
        contract: Address,
        from_block: u64,
        to_block: u64,
    ) -> Result<BoxedCommitmentStream<'_>, SyncerError> {
        let mut streams: Vec<BoxedCommitmentStream<'_>> = Vec::new();
        let mut current_from = from_block;

        for (i, syncer) in self.syncers.iter().enumerate() {
            if current_from > to_block {
                break;
            }

            let syncer_latest = syncer.latest_block().await?;
            if syncer_latest < current_from {
                continue;
            }

            let range_end = syncer_latest.min(to_block);
            match syncer
                .sync_commitments(contract, current_from, range_end)
                .await
            {
                Ok(stream) => streams.push(stream),
                Err(e) => {
                    tracing::warn!("Syncer {} failed: {}", i, e);
                }
            }

            current_from = range_end + 1;
        }
        let combined = stream::iter(streams).flatten();
        Ok(Box::pin(combined))
    }

    async fn sync_nullifiers(
        &self,
        contract: Address,
        from_block: u64,
        to_block: u64,
    ) -> Result<BoxedNullifierStream<'_>, SyncerError> {
        let mut streams: Vec<BoxedNullifierStream<'_>> = Vec::new();
        let mut current_from = from_block;

        for (i, syncer) in self.syncers.iter().enumerate() {
            if current_from > to_block {
                break;
            }

            let syncer_latest = syncer.latest_block().await?;
            if syncer_latest < current_from {
                continue;
            }

            let range_end = syncer_latest.min(to_block);
            match syncer
                .sync_nullifiers(contract, current_from, range_end)
                .await
            {
                Ok(stream) => streams.push(stream),
                Err(e) => {
                    tracing::warn!("Syncer {} failed: {}", i, e);
                }
            }

            current_from = range_end + 1;
        }
        let combined = stream::iter(streams).flatten();
        Ok(Box::pin(combined))
    }
}
