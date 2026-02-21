use std::sync::Arc;

use alloy::providers::DynProvider;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    chain_config::ChainConfig,
    circuit::prover::{PoiProver, TransactProver},
    railgun::{
        broadcaster::broadcaster::Broadcaster,
        indexer::{
            TxidIndexer, TxidIndexerError, TxidIndexerState,
            syncer::{NoteSyncer, TransactionSyncer},
        },
        merkle_tree::MerkleTreeVerifier,
        poi::{
            PendingPoiError, PendingPoiSubmitter, PoiClient,
            pending_poi_submitter::PendingPoiSubmitterState,
        },
        provider::{RailgunProvider, RailgunProviderError, RailgunProviderState},
        signer::Signer,
        transaction::{ShieldBuilder, TransactionBuilder, WithBroadcast, WithPoi},
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
}

impl PoiProvider {
    pub fn new(
        chain: ChainConfig,
        provider: DynProvider,
        utxo_syncer: Arc<dyn NoteSyncer>,
        utxo_verifier: Arc<dyn MerkleTreeVerifier>,
        tx_prover: Arc<dyn TransactProver>,
        txid_syncer: Arc<dyn TransactionSyncer>,
        poi_client: PoiClient,
        poi_prover: Arc<dyn PoiProver>,
    ) -> Self {
        Self {
            inner: RailgunProvider::new(
                chain,
                provider.clone(),
                utxo_syncer,
                utxo_verifier,
                tx_prover,
            ),
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
        utxo_verifier: Arc<dyn MerkleTreeVerifier>,
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
                utxo_verifier,
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

    /// Returns POI augmented balance, with metadata on the POI status for notes
    pub fn balance(&self) {
        todo!()
    }

    pub fn shield(&self) -> ShieldBuilder {
        self.inner.shield()
    }

    pub fn transact(&self) -> TransactionBuilder<'_, WithPoi> {
        self.inner
            .transact()
            .with_poi(&self.poi_client, self.prover.as_ref())
    }

    pub fn transact_broadcast(
        &mut self,
        fee_payer: Arc<dyn Signer>,
        broadcaster: Broadcaster,
    ) -> TransactionBuilder<'_, WithBroadcast> {
        self.inner.transact().with_broadcast(
            &self.poi_client,
            self.prover.as_ref(),
            &self.provider,
            fee_payer,
            broadcaster,
            &mut self.pending_submitter,
        )
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
}
