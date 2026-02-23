use std::{collections::HashMap, pin::pin, sync::Arc};

use alloy::providers::DynProvider;
use futures::future::Either;
use rand::Rng;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::info;

use crate::{
    caip::AssetId,
    chain_config::ChainConfig,
    circuit::prover::{PoiProver, TransactProver},
    railgun::{
        address::RailgunAddress,
        broadcaster::broadcaster::{BroadcastError, Broadcaster, Fee},
        indexer::{
            NoteSyncer, TransactionSyncer, TxidIndexer, TxidIndexerError, TxidIndexerState,
            UtxoIndexerError,
        },
        poi::{
            PendingPoiError, PendingPoiSubmitter, PoiClient,
            pending_poi_submitter::PendingPoiSubmitterState,
        },
        provider::{RailgunProvider, RailgunProviderError, RailgunProviderState},
        signer::Signer,
        transaction::{
            PoiProvedTx, PoiTransactionBuilder, PoiTransactionBuilderError, ShieldBuilder,
        },
    },
    sleep::sleep,
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
    Build(#[from] PoiTransactionBuilderError),
    #[error("Broadcast error: {0}")]
    Broadcast(#[from] BroadcastError),
    #[error("Timed out waiting for operation to land on-chain")]
    Timeout,
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
    // pub fn balance(&self, address: RailgunAddress) -> HashMap<PoiStatus, HashMap<AssetId, u128>> {
    //     // let notes = self.inner.utxo_indexer.unspent(address);
    //     // let poi_notes =
    // }

    pub fn balance(&self, address: RailgunAddress) -> HashMap<AssetId, u128> {
        self.inner.balance(address)
    }

    pub fn shield(&self) -> ShieldBuilder {
        self.inner.shield()
    }

    pub fn transact(&self) -> PoiTransactionBuilder {
        PoiTransactionBuilder::new()
    }

    pub async fn build<R: Rng>(
        &self,
        builder: PoiTransactionBuilder,
        rng: &mut R,
    ) -> Result<PoiProvedTx, PoiProviderError> {
        Ok(builder
            .build_poi(
                self.inner.chain.clone(),
                &self.inner.utxo_indexer,
                self.inner.prover.as_ref(),
                &self.poi_client,
                self.prover.as_ref(),
                rng,
            )
            .await?)
    }

    pub async fn build_broadcast<R: Rng>(
        &mut self,
        builder: PoiTransactionBuilder,
        fee_payer: Arc<dyn Signer>,
        fee: &Fee,
        rng: &mut R,
    ) -> Result<PoiProvedTx, PoiProviderError> {
        let tx = builder
            .build_broadcast(
                self.inner.chain.clone(),
                &self.inner.utxo_indexer,
                self.inner.prover.as_ref(),
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

    /// Broadcasts a transaction and races the broadcaster response against
    /// the UTXO commitment appearing on-chain. Whichever completes first
    /// wins; the other future is dropped (cancelled). After the race,
    /// runs a full sync (txid indexer + pending POI submission).
    pub async fn broadcast(
        &mut self,
        broadcaster: &Broadcaster,
        tx: &PoiProvedTx,
    ) -> Result<(), PoiProviderError> {
        let commitments: Vec<_> = tx
            .operations
            .iter()
            .flat_map(|op| &op.circuit_inputs.commitments_out)
            .copied()
            .collect();

        {
            let mut rng = rand::rng();
            let broadcast_fut = pin!(broadcaster.broadcast(tx, &mut rng));
            let await_fut = pin!(self.inner.utxo_indexer.await_commitments(
                &commitments,
                web_time::Duration::from_secs(5),
                web_time::Duration::from_secs(120),
            ));

            match futures::future::select(broadcast_fut, await_fut).await {
                Either::Left((Ok(tx_hash), _)) => {
                    info!("Confirmed via broadcaster response: {tx_hash}");
                }
                Either::Left((Err(e), _)) => return Err(e.into()),
                Either::Right((Ok(()), _)) => {
                    info!("Confirmed via indexer (commitment found on-chain)");
                }
                Either::Right((Err(e), _)) => {
                    return Err(match e {
                        UtxoIndexerError::Timeout => PoiProviderError::Timeout,
                        other => PoiProviderError::RailgunProvider(other.into()),
                    });
                }
            }
        }

        self.sync().await?;
        Ok(())
    }

    pub async fn await_indexed(&mut self, tx: &PoiProvedTx) -> Result<(), PoiProviderError> {
        let mut remaining_txids: Vec<_> = tx.operations.iter().filter_map(|op| op.txid).collect();

        loop {
            let Some(txid) = remaining_txids.last() else {
                break;
            };

            if self.txid_indexer.txid_position(txid).is_some() {
                remaining_txids.pop();
            } else {
                info!("Waiting for txid {:?} to be indexed...", txid);
                sleep(web_time::Duration::from_secs(5)).await;
            }

            info!("Sycning...");
            self.sync().await?;
        }

        Ok(())
    }

    pub async fn sync(&mut self) -> Result<(), PoiProviderError> {
        self.inner.sync().await?;
        self.txid_indexer.sync().await?;
        self.pending_submitter
            .process(
                &self.txid_indexer,
                &self.inner.utxo_indexer,
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
                &self.inner.utxo_indexer,
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
