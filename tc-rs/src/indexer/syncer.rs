use std::pin::Pin;

use alloy::primitives::{Address, FixedBytes, TxHash};
use futures::Stream;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SyncerError {
    #[error("Syncer error: {0}")]
    Syncer(#[from] Box<dyn std::error::Error>),
    #[error("Invalid contract {contract}: {reason}")]
    InvalidContract { contract: Address, reason: String },
}

#[cfg_attr(not(feature = "wasm"), async_trait::async_trait)]
#[cfg_attr(feature = "wasm", async_trait::async_trait(?Send))]
pub trait Syncer: Send + Sync {
    async fn latest_block(&self) -> Result<u64, SyncerError>;

    async fn sync_commitments(
        &self,
        contract: Address,
        from_block: u64,
        to_block: u64,
    ) -> Result<BoxedCommitmentStream<'_>, SyncerError>;

    async fn sync_nullifiers(
        &self,
        contract: Address,
        from_block: u64,
        to_block: u64,
    ) -> Result<BoxedNullifierStream<'_>, SyncerError>;
}

pub struct Commitment {
    pub block_number: u64,
    pub tx_hash: TxHash,
    pub commitment: FixedBytes<32>,
    pub leaf_index: u32,
    pub timestamp: u64,
}

pub struct Nullifier {
    pub block_number: u64,
    pub tx_hash: TxHash,
    pub nullifier: FixedBytes<32>,
    pub to: Address,
    pub fee: u128,
    pub timestamp: u64,
}

#[cfg(not(feature = "wasm"))]
pub type BoxedCommitmentStream<'a> = Pin<Box<dyn Stream<Item = Commitment> + Send + 'a>>;

#[cfg(feature = "wasm")]
pub type BoxedCommitmentStream<'a> = Pin<Box<dyn Stream<Item = Commitment> + 'a>>;

#[cfg(not(feature = "wasm"))]
pub type BoxedNullifierStream<'a> = Pin<Box<dyn Stream<Item = Nullifier> + Send + 'a>>;

#[cfg(feature = "wasm")]
pub type BoxedNullifierStream<'a> = Pin<Box<dyn Stream<Item = Nullifier> + 'a>>;
