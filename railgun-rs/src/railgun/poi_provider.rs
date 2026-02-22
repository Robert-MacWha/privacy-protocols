use std::{collections::HashMap, sync::Arc};

use alloy::providers::DynProvider;
use rand::Rng;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    caip::AssetId,
    chain_config::ChainConfig,
    circuit::prover::{PoiProver, TransactProver},
    railgun::{
        address::RailgunAddress,
        broadcaster::broadcaster::Fee,
        indexer::{
            TxidIndexer, TxidIndexerError, TxidIndexerState,
            syncer::{NoteSyncer, TransactionSyncer},
        },
        poi::{
            PendingPoiError, PendingPoiSubmitter, PoiClient,
            pending_poi_submitter::PendingPoiSubmitterState,
        },
        provider::{RailgunProvider, RailgunProviderError, RailgunProviderState},
        signer::Signer,
        transaction::{BuildError, PoiProvedTx, ShieldBuilder, TransactionBuilder},
    },
};

pub struct PoiProvider {
    inner: RailgunProvider,

    provider: DynProvider,
    txid_indexer: TxidIndexer,
    poi_client: PoiClient,
    prover: Arc<dyn PoiProver>,
    pending_submitter: PendingPoiSubmitter,
}

#[derive(Serialize, Deserialize)]
pub struct PoiProviderState {
    pub inner: RailgunProviderState,
    pub txid_indexer: TxidIndexerState,
    pub pending_submitter: PendingPoiSubmitterState,
}

#[derive(Debug, Error)]
pub enum PoiProviderError {
    #[error("Railgun provider error: {0}")]
    RailgunProvider(#[from] RailgunProviderError),
    #[error("Txid indexer error: {0}")]
    TxidIndexer(#[from] TxidIndexerError),
    #[error("Pending POI error: {0}")]
    PoiClient(#[from] PendingPoiError),
    #[error("Build error: {0}")]
    Build(#[from] BuildError),
}

impl PoiProvider {
    pub fn new(
        chain: ChainConfig,
        provider: DynProvider,
        utxo_syncer: Arc<dyn NoteSyncer>,
        tx_prover: Arc<dyn TransactProver>,
        txid_syncer: Arc<dyn TransactionSyncer>,
        poi_client: PoiClient,
        poi_prover: Arc<dyn PoiProver>,
    ) -> Self {
        Self {
            inner: RailgunProvider::new(chain, provider.clone(), utxo_syncer, tx_prover),
            provider,
            txid_indexer: TxidIndexer::new(txid_syncer, poi_client.clone()),
            poi_client,
            prover: poi_prover,
            pending_submitter: PendingPoiSubmitter::new(),
        }
    }

    pub fn from_state(
        state: PoiProviderState,
        provider: DynProvider,
        utxo_syncer: Arc<dyn NoteSyncer>,
        tx_prover: Arc<dyn TransactProver>,
        txid_syncer: Arc<dyn TransactionSyncer>,
        poi_client: PoiClient,
        poi_prover: Arc<dyn PoiProver>,
    ) -> Result<Self, PoiProviderError> {
        Ok(Self {
            inner: RailgunProvider::from_state(
                state.inner,
                provider.clone(),
                utxo_syncer,
                tx_prover,
            )?,
            provider,
            txid_indexer: TxidIndexer::from_state(
                txid_syncer,
                poi_client.clone(),
                state.txid_indexer,
            ),
            poi_client,
            prover: poi_prover,
            pending_submitter: PendingPoiSubmitter::from_state(state.pending_submitter),
        })
    }

    pub fn state(&self) -> PoiProviderState {
        PoiProviderState {
            inner: self.inner.state(),
            txid_indexer: self.txid_indexer.state(),
            pending_submitter: self.pending_submitter.state(),
        }
    }

    pub fn register(&mut self, account: Arc<dyn Signer>) {
        self.inner.register(account);
    }

    /// Returns POI augmented balance, with metadata on the POI status for notes
    pub fn balance(&self, address: RailgunAddress) -> HashMap<AssetId, u128> {
        self.inner.balance(address)
    }

    pub fn shield(&self) -> ShieldBuilder {
        self.inner.shield()
    }

    pub fn transact(&self) -> TransactionBuilder {
        self.inner.transact()
    }

    pub async fn build<R: Rng>(
        &self,
        builder: TransactionBuilder,
        rng: &mut R,
    ) -> Result<PoiProvedTx, PoiProviderError> {
        Ok(builder
            .build_poi(
                self.inner.chain.clone(),
                &self.inner.utxo_indexer(),
                self.inner.prover().as_ref(),
                &self.poi_client,
                self.prover.as_ref(),
                rng,
            )
            .await?)
    }

    pub async fn build_broadcast<R: Rng>(
        &mut self,
        builder: TransactionBuilder,
        fee_payer: Arc<dyn Signer>,
        fee: &Fee,
        rng: &mut R,
    ) -> Result<PoiProvedTx, PoiProviderError> {
        let tx = builder
            .build_broadcast(
                self.inner.chain.clone(),
                &self.inner.utxo_indexer(),
                self.inner.prover().as_ref(),
                &self.poi_client,
                self.prover.as_ref(),
                &self.provider,
                fee_payer,
                fee,
                rng,
            )
            .await?;

        for op in &tx.operations {
            self.pending_submitter.register(op);
        }

        Ok(tx)
    }

    pub async fn sync(&mut self) -> Result<(), PoiProviderError> {
        self.inner.sync().await?;
        self.txid_indexer.sync().await?;
        self.pending_submitter
            .process(
                &self.txid_indexer,
                self.inner.utxo_indexer(),
                &self.poi_client,
                self.prover.as_ref(),
            )
            .await?;
        Ok(())
    }

    pub async fn sync_to(&mut self, block_number: u64) -> Result<(), PoiProviderError> {
        self.inner.sync_to(block_number).await?;
        self.txid_indexer.sync_to(block_number).await?;
        self.pending_submitter
            .process(
                &self.txid_indexer,
                self.inner.utxo_indexer(),
                &self.poi_client,
                self.prover.as_ref(),
            )
            .await?;
        Ok(())
    }

    /// Resets the provider's internal indexer state
    pub fn reset_indexer(&mut self) {
        self.inner.reset_indexer();
        self.txid_indexer.reset();
    }
}
