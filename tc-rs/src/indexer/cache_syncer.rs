use std::path::Path;

use alloy::primitives::{Address, FixedBytes, TxHash};
use futures::stream;
use serde::Deserialize;
use thiserror::Error;
use tracing::info;

use crate::indexer::syncer::{
    BoxedCommitmentStream, BoxedNullifierStream, Commitment, Nullifier, Syncer, SyncerError,
};

#[derive(Debug, Error)]
pub enum CacheSyncerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct CacheSyncer {
    commitments: Vec<Commitment>,
    nullifiers: Vec<Nullifier>,
}

#[derive(Deserialize)]
struct RawDeposit {
    #[serde(rename = "blockNumber")]
    block_number: u64,
    #[serde(rename = "transactionHash")]
    transaction_hash: TxHash,
    commitment: FixedBytes<32>,
    #[serde(rename = "leafIndex")]
    leaf_index: u32,
}

#[derive(Deserialize)]
struct RawWithdrawal {
    #[serde(rename = "blockNumber")]
    block_number: u64,
    #[serde(rename = "transactionHash")]
    transaction_hash: TxHash,
    #[serde(rename = "nullifierHash")]
    nullifier_hash: FixedBytes<32>,
    to: Address,
    fee: String,
}

impl CacheSyncer {
    pub fn from_files(
        deposits_path: &Path,
        withdrawals_path: &Path,
    ) -> Result<Self, CacheSyncerError> {
        let deposits_file = std::fs::File::open(deposits_path)?;
        let withdrawals_file = std::fs::File::open(withdrawals_path)?;

        let raw_deposits: Vec<RawDeposit> = serde_json::from_reader(deposits_file)?;
        let raw_withdrawals: Vec<RawWithdrawal> = serde_json::from_reader(withdrawals_file)?;

        Self::from_raw(raw_deposits, raw_withdrawals)
    }

    pub fn from_json(
        deposits_json: &str,
        nullifiers_json: &str,
    ) -> Result<Self, CacheSyncerError> {
        let raw_deposits: Vec<RawDeposit> = serde_json::from_str(deposits_json)?;
        let raw_withdrawals: Vec<RawWithdrawal> = serde_json::from_str(nullifiers_json)?;
        Self::from_raw(raw_deposits, raw_withdrawals)
    }

    fn from_raw(
        raw_deposits: Vec<RawDeposit>,
        raw_withdrawals: Vec<RawWithdrawal>,
    ) -> Result<Self, CacheSyncerError> {
        let commitments = raw_deposits
            .into_iter()
            .map(|d| Commitment {
                block_number: d.block_number,
                tx_hash: d.transaction_hash,
                commitment: d.commitment,
                leaf_index: d.leaf_index,
                timestamp: 0,
            })
            .collect();

        let nullifiers = raw_withdrawals
            .into_iter()
            .map(|w| Nullifier {
                block_number: w.block_number,
                tx_hash: w.transaction_hash,
                nullifier: w.nullifier_hash,
                to: w.to,
                fee: w.fee.parse::<u128>().unwrap_or(0),
                timestamp: 0,
            })
            .collect();

        Ok(Self {
            commitments,
            nullifiers,
        })
    }
}

#[cfg_attr(not(feature = "wasm"), async_trait::async_trait)]
#[cfg_attr(feature = "wasm", async_trait::async_trait(?Send))]
impl Syncer for CacheSyncer {
    async fn latest_block(&self) -> Result<u64, SyncerError> {
        let max_commitment = self
            .commitments
            .iter()
            .map(|c| c.block_number)
            .max()
            .unwrap_or(0);
        let max_nullifier = self
            .nullifiers
            .iter()
            .map(|n| n.block_number)
            .max()
            .unwrap_or(0);
        Ok(max_commitment.max(max_nullifier))
    }

    async fn sync_commitments(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<BoxedCommitmentStream<'_>, SyncerError> {
        info!(
            "CacheSyncer syncing commitments from block {} to {}",
            from_block, to_block
        );

        let items: Vec<Commitment> = self
            .commitments
            .iter()
            .filter(|c| c.block_number >= from_block && c.block_number <= to_block)
            .map(|c| Commitment {
                block_number: c.block_number,
                tx_hash: c.tx_hash,
                commitment: c.commitment,
                leaf_index: c.leaf_index,
                timestamp: c.timestamp,
            })
            .collect();
        Ok(Box::pin(stream::iter(items)))
    }

    async fn sync_nullifiers(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<BoxedNullifierStream<'_>, SyncerError> {
        info!(
            "CacheSyncer syncing nullifiers from block {} to {}",
            from_block, to_block
        );

        let items: Vec<Nullifier> = self
            .nullifiers
            .iter()
            .filter(|n| n.block_number >= from_block && n.block_number <= to_block)
            .map(|n| Nullifier {
                block_number: n.block_number,
                tx_hash: n.tx_hash,
                nullifier: n.nullifier,
                to: n.to,
                fee: n.fee,
                timestamp: n.timestamp,
            })
            .collect();
        Ok(Box::pin(stream::iter(items)))
    }
}
