use std::{collections::HashMap, sync::Arc};

use alloy::{primitives::ChainId, providers::DynProvider};
use rand::Rng;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    caip::AssetId,
    chain_config::{ChainConfig, get_chain_config},
    circuit::prover::TransactProver,
    railgun::{
        address::RailgunAddress,
        indexer::{NoteSyncer, UtxoIndexer, UtxoIndexerError, UtxoIndexerState},
        merkle_tree::SmartWalletUtxoVerifier,
        signer::Signer,
        transaction::{ProvedTx, ShieldBuilder, TransactionBuilder, TransactionBuilderError},
    },
};

/// Provides access to Railgun interactions
pub struct RailgunProvider {
    pub chain: ChainConfig,
    pub(crate) utxo_indexer: UtxoIndexer,
    pub(crate) prover: Arc<dyn TransactProver>,
}

#[derive(Serialize, Deserialize)]
pub struct RailgunProviderState {
    pub chain_id: ChainId,
    pub indexer: UtxoIndexerState,
}

#[derive(Debug, Error)]
pub enum RailgunProviderError {
    #[error("Unsupported chain ID: {0}")]
    UnsupportedChainId(ChainId),
    #[error("Utxo indexer error: {0}")]
    UtxoIndexer(#[from] UtxoIndexerError),
    #[error("Build error: {0}")]
    Build(#[from] TransactionBuilderError),
}

/// General provider functions
impl RailgunProvider {
    pub fn new(
        chain: ChainConfig,
        provider: DynProvider,
        utxo_syncer: Arc<dyn NoteSyncer>,
        prover: Arc<dyn TransactProver>,
    ) -> Self {
        let utxo_verifier = Arc::new(SmartWalletUtxoVerifier::new(
            chain.railgun_smart_wallet,
            provider.clone(),
        ));

        Self {
            chain,
            utxo_indexer: UtxoIndexer::new(utxo_syncer, utxo_verifier),
            prover,
        }
    }

    pub fn from_state(
        state: RailgunProviderState,
        provider: DynProvider,
        utxo_syncer: Arc<dyn NoteSyncer>,
        prover: Arc<dyn TransactProver>,
    ) -> Result<Self, RailgunProviderError> {
        let chain = get_chain_config(state.chain_id)
            .ok_or(RailgunProviderError::UnsupportedChainId(state.chain_id))?;

        let utxo_verifier = Arc::new(SmartWalletUtxoVerifier::new(
            chain.railgun_smart_wallet,
            provider.clone(),
        ));

        Ok(Self {
            chain,
            utxo_indexer: UtxoIndexer::from_state(utxo_syncer, utxo_verifier, state.indexer),
            prover,
        })
    }

    pub fn state(&self) -> RailgunProviderState {
        RailgunProviderState {
            chain_id: self.chain.id,
            indexer: self.utxo_indexer.state(),
        }
    }

    /// Registers an account with the provider. The provider will track the balance
    /// and transactions for this account as it syncs.
    pub fn register(&mut self, account: Arc<dyn Signer>) {
        self.utxo_indexer.register(account);
    }

    /// Registers an account and resyncs from the specified block. Resyncing is
    /// necessary to initially populate an account's state. Resyncing can be skipped
    ///
    pub async fn register_resync(
        &mut self,
        account: Arc<dyn Signer>,
        from_block: Option<u64>,
    ) -> Result<(), RailgunProviderError> {
        self.utxo_indexer
            .register_resync(account, from_block)
            .await?;
        Ok(())
    }

    /// Raw railgun balance
    pub fn balance(&self, address: RailgunAddress) -> HashMap<AssetId, u128> {
        self.utxo_indexer.balance(address)
    }

    /// Returns a shield builder
    pub fn shield(&self) -> ShieldBuilder {
        ShieldBuilder::new(self.chain)
    }

    /// Returns a transact builder
    pub fn transact(&self) -> TransactionBuilder {
        TransactionBuilder::new()
    }

    /// Builds a transaction using the provider's internal state
    pub async fn build<R: Rng>(
        &self,
        builder: TransactionBuilder,
        rng: &mut R,
    ) -> Result<ProvedTx, RailgunProviderError> {
        Ok(builder
            .build(
                self.chain.clone(),
                &self.utxo_indexer,
                self.prover.as_ref(),
                rng,
            )
            .await?)
    }

    /// Syncs the provider to the specified block number.
    pub async fn sync_to(&mut self, block_number: u64) -> Result<(), RailgunProviderError> {
        self.utxo_indexer.sync_to(block_number).await?;
        Ok(())
    }

    /// Syncs the provider to the latest block.
    pub async fn sync(&mut self) -> Result<(), RailgunProviderError> {
        self.utxo_indexer.sync().await?;
        Ok(())
    }

    /// Resets the provider's internal indexer state
    pub fn reset_indexer(&mut self) {
        self.utxo_indexer.reset();
    }
}
