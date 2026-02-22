use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use alloy::primitives::Address;
use rand::Rng;
use thiserror::Error;
use tracing::info;

use crate::{
    caip::AssetId,
    chain_config::ChainConfig,
    circuit::{
        inputs::PoiCircuitInputsError,
        prover::{PoiProver, TransactProver},
    },
    railgun::{
        Signer,
        address::RailgunAddress,
        broadcaster::broadcaster::Fee,
        indexer::UtxoIndexer,
        merkle_tree::{MerkleRoot, UtxoMerkleTree},
        note::{operation::Operation, utxo::UtxoNote},
        poi::{ListKey, PoiClient, PoiClientError},
        transaction::{
            GasEstimator, PoiProvedOperation, PoiProvedOperationError, PoiProvedTx, ProvedTx,
            TransactionBuilder, TransactionBuilderError,
            transaction_builder::{build_operations, prove_operations},
        },
    },
};

pub struct PoiTransactionBuilder {
    inner: TransactionBuilder,
}

#[derive(Debug, Error)]
pub enum PoiTransactionBuilderError {
    #[error("Poi Client error: {0}")]
    PoiClient(#[from] PoiClientError),
    #[error("Poi Circuit input error: {0}")]
    PoiCircuitInput(#[from] PoiCircuitInputsError),
    #[error("POI Proved Operation error: {0}")]
    PoiProvedOperation(#[from] PoiProvedOperationError),
    #[error("Invalid POI merkleroot for list key {0}: {1}")]
    InvalidPoiMerkleroot(ListKey, MerkleRoot),
    #[error("Estimator error: {0}")]
    Estimator(Box<dyn std::error::Error>),
    #[error("Build error: {0}")]
    Build(#[from] TransactionBuilderError),
}

const FEE_BUFFER: f64 = 1.3;

impl PoiTransactionBuilder {
    pub fn new() -> Self {
        Self {
            inner: TransactionBuilder::new(),
        }
    }

    pub fn transfer(
        self,
        from: Arc<dyn Signer>,
        to: RailgunAddress,
        asset: AssetId,
        value: u128,
        memo: &str,
    ) -> Self {
        Self {
            inner: self.inner.transfer(from, to, asset, value, memo),
        }
    }

    pub fn set_unshield(
        self,
        from: Arc<dyn Signer>,
        to: Address,
        asset: AssetId,
        value: u128,
    ) -> Self {
        Self {
            inner: self.inner.set_unshield(from, to, asset, value),
        }
    }

    /// Builds and proves a transaction for railgun.
    ///
    /// The resulting transaction can be self-broadcasted and includes POI Proof
    /// data.
    pub async fn build_poi<R: Rng>(
        self,
        chain: ChainConfig,
        indexer: &UtxoIndexer,
        prover: &dyn TransactProver,
        poi_client: &PoiClient,
        poi_prover: &dyn PoiProver,
        rng: &mut R,
    ) -> Result<PoiProvedTx, PoiTransactionBuilderError> {
        let in_notes = indexer.all_unspent();
        let draft = self.inner.draft_operations(rng);
        let ops = build_operations(draft, in_notes, rng)?;
        let proved = prove_operations(prover, &indexer.utxo_trees, &ops, chain, 0, rng).await?;

        let list_keys = poi_client.list_keys();
        self.prove_poi(
            poi_prover,
            poi_client,
            proved,
            &indexer.utxo_trees,
            &list_keys,
            None,
        )
        .await
    }

    /// Builds a transaction with fee calculation and POI proofs for broadcasting.
    ///
    /// The resulting transaction includes POI proof data and a broadcaster fee, and is
    /// ready for broadcasting with the provided broadcaster.
    pub async fn build_broadcast<R: Rng>(
        self,
        chain: ChainConfig,
        indexer: &UtxoIndexer,
        prover: &dyn TransactProver,
        poi_client: &PoiClient,
        poi_prover: &dyn PoiProver,
        estimator: &dyn GasEstimator,
        fee_payer: Arc<dyn Signer>,
        fee: &Fee,
        rng: &mut R,
    ) -> Result<PoiProvedTx, PoiTransactionBuilderError> {
        let in_notes = indexer.all_unspent();

        let proved = self
            .calculate_fee_to_convergence(
                &in_notes,
                prover,
                &indexer.utxo_trees,
                estimator,
                fee_payer.clone(),
                &fee,
                chain,
                rng,
            )
            .await?;

        let tx = self
            .prove_poi(
                poi_prover,
                poi_client,
                proved,
                &indexer.utxo_trees,
                &fee.list_keys,
                Some(fee.clone()),
            )
            .await?;

        Ok(tx)
    }

    /// Calculate fee iteratively until convergence. It iteratively builds and proves
    /// transactions until the fee converges to a stable value.
    async fn calculate_fee_to_convergence<R: Rng>(
        &self,
        in_notes: &[UtxoNote],
        prover: &dyn TransactProver,
        utxo_trees: &BTreeMap<u32, UtxoMerkleTree>,
        estimator: &dyn GasEstimator,
        fee_payer: Arc<dyn Signer>,
        fee: &Fee,
        chain: ChainConfig,
        rng: &mut R,
    ) -> Result<ProvedTx, PoiTransactionBuilderError> {
        const MAX_ITERS: usize = 5;

        let gas_price_wei = estimator
            .gas_price_wei()
            .await
            .map_err(PoiTransactionBuilderError::Estimator)?;

        let fee_builder = self.inner.clone();
        let mut last_fee: u128 = calculate_fee(1000000, gas_price_wei, fee.per_unit_gas);

        //? Create draft fee note as the last transfer, so we know where to edit
        //? when iterating
        let fee_asset = AssetId::Erc20(fee.token);
        let mut fee_builder = fee_builder.transfer(
            fee_payer.clone(),
            fee.recipient.clone(),
            fee_asset,
            last_fee,
            "fee",
        );

        let mut proved_tx: Option<ProvedTx> = None;
        for _ in 0..MAX_ITERS {
            let draft = fee_builder.draft_operations(rng);
            let mut operations = build_operations(draft, in_notes.to_vec(), rng)?;

            // Sort operations so the fee operation is first
            operations.sort_by_key(|op| {
                !(op.from.address() == fee_payer.address() && op.asset == fee_asset)
            });

            // Sort the out notes of the fee operation so the fee note is first
            operations.first_mut().map(|op| {
                op.out_notes.sort_by_key(|n| {
                    !(n.from_key == fee_payer.viewing_key() && n.asset == fee_asset)
                });
            });

            //? Sanity check that the fee operation is first and has the correct asset
            debug_assert!(
                operations
                    .first()
                    .map(|op| op.from.address() == fee_payer.address() && op.asset == fee_asset)
                    .unwrap_or(false)
            );

            let proved = prove_operations(prover, utxo_trees, &operations, chain, 0, rng).await?;

            let gas = estimator
                .estimate_gas(&proved.tx_data)
                .await
                .map_err(PoiTransactionBuilderError::Estimator)?;

            proved_tx = Some(proved);
            let new_fee = calculate_fee(gas, gas_price_wei, fee.per_unit_gas);

            info!(
                "Estimated gas: {}, gas price (wei): {}, fee: {}",
                gas, gas_price_wei, new_fee
            );
            if new_fee <= last_fee {
                info!("Fee converged at {} after iterations", new_fee);
                break;
            }

            //? Safe since we know the fee transfer is always the last transfer
            //? in the builder
            fee_builder.transfers.last_mut().unwrap().value = new_fee;
            last_fee = new_fee;
        }

        //? Safe since we're always assigning proved_tx in the above loop
        let mut proved = proved_tx.unwrap();
        proved.min_gas_price = gas_price_wei;
        Ok(proved)
    }

    /// Attach POI proofs to a proved transaction.
    async fn prove_poi(
        &self,
        poi_prover: &dyn PoiProver,
        poi_client: &PoiClient,
        proved: ProvedTx,
        utxo_trees: &BTreeMap<u32, UtxoMerkleTree>,
        list_keys: &[ListKey],
        fee: Option<Fee>,
    ) -> Result<PoiProvedTx, PoiTransactionBuilderError> {
        // Rebuild operations with PoiNote inputs (needed for POI merkle proofs)
        let proved_operations = proved.proved_operations;
        let mut poi_operations = Vec::new();
        for operation in proved_operations {
            let op = operation.operation;
            let in_notes = op.in_notes;
            let poi_in_notes = poi_client.note_to_poi_note(in_notes, list_keys).await?;

            //? Need to create a new operation since the generic can't be
            //? trivially cast.
            poi_operations.push(PoiProvedOperation {
                operation: Operation {
                    utxo_tree_number: op.utxo_tree_number,
                    from: op.from,
                    asset: op.asset,
                    in_notes: poi_in_notes,
                    out_notes: op.out_notes,
                    unshield_note: op.unshield_note,
                },
                circuit_inputs: operation.circuit_inputs,
                transaction: operation.transaction,
                pois: HashMap::new(),
                txid_leaf_hash: None,
                txid: None,
            });
        }

        // Attach POI proofs to each operation
        for poi_op in poi_operations.iter_mut() {
            poi_op.add_pois(poi_prover, list_keys, utxo_trees).await?;
        }

        // Validate all POI merkle roots
        //? Should always pass, but sanity check to ensure proofs are valid before broadcasting
        #[cfg(debug_assertions)]
        for poi_op in poi_operations.iter() {
            for (list_key, poi) in poi_op.pois.iter() {
                for merkleroot in &poi.poi_merkleroots {
                    let valid = poi_client
                        .validate_poi_merkleroot(list_key.clone(), *merkleroot)
                        .await?;
                    if !valid {
                        return Err(PoiTransactionBuilderError::InvalidPoiMerkleroot(
                            list_key.clone(),
                            *merkleroot,
                        ));
                    }
                }
            }
        }

        Ok(PoiProvedTx {
            tx_data: proved.tx_data,
            operations: poi_operations,
            min_gas_price: proved.min_gas_price,
            fee,
        })
    }
}

/// Calculate the broadcaster's fee based on the estimated gas cost, gas price in wei,
/// broadcaster's fee rate, and a buffer.
fn calculate_fee(gas_cost: u128, gas_price_wei: u128, fee_rate: u128) -> u128 {
    let raw = (gas_cost * gas_price_wei * fee_rate) / 10_u128.pow(18);
    ((raw as f64) * FEE_BUFFER).ceil() as u128
}
