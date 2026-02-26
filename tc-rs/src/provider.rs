use std::sync::Arc;

use alloy::primitives::{Address, Bytes};
use alloy_sol_types::SolCall;
use prover::{Proof, Prover};
use rand::Rng;
use ruint::aliases::U256;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::info;

use crate::{
    abis::tornado::Tornado::{self},
    circuit::{WithdrawCircuitInputs, WithdrawCircuitInputsError},
    indexer::{Indexer, IndexerError, IndexerState},
    note::Note,
    pool::{Asset, Pool},
    tx_data::TxData,
};

pub struct TornadoProvider {
    chain_id: u64,
    pub indexer: Indexer,
    prover: Arc<dyn Prover>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TornadoProviderState {
    pub chain_id: u64,
    pub indexer_state: IndexerState,
}

#[derive(Debug, Error)]
pub enum TornadoProviderError {
    #[error("Indexer error: {0}")]
    Indexer(#[from] IndexerError),
    #[error("Withdraw circuit inputs error: {0}")]
    WithdrawCircuitInputs(#[from] WithdrawCircuitInputsError),
    #[error("Prover error: {0}")]
    Prover(#[from] prover::ProverError),
}

impl TornadoProvider {
    pub fn new(chain_id: u64, indexer: Indexer, prover: Arc<dyn Prover>) -> Self {
        Self {
            chain_id,
            indexer,
            prover,
        }
    }

    pub fn state(&self) -> TornadoProviderState {
        TornadoProviderState {
            chain_id: self.chain_id,
            indexer_state: self.indexer.state(),
        }
    }

    pub fn set_state(&mut self, state: TornadoProviderState) {
        self.chain_id = state.chain_id;
        self.indexer.set_state(state.indexer_state);
    }

    pub fn deposit<R: Rng>(&self, pool: &dyn Pool, rng: &mut R) -> (TxData, Note) {
        let note = Note::random(rng, &pool.symbol(), &pool.amount(), self.chain_id);

        let call = Tornado::depositCall {
            _commitment: note.commitment().into(),
        };
        let calldata = call.abi_encode();

        let value = match pool.asset() {
            Asset::Native { .. } => pool.amount_wei(),
            Asset::Erc20 { .. } => 0,
        };

        let tx_data = TxData {
            to: pool.address(),
            data: calldata,
            value: U256::from(value),
        };
        (tx_data, note)
    }

    pub async fn withdraw(
        &self,
        pool: &dyn Pool,
        note: &Note,
        recipient: Address,
        relayer: Address,
        fee: U256,
        refund: U256,
    ) -> Result<TxData, TornadoProviderError> {
        let merkle_tree = self.indexer.tree();
        let circuit_inputs =
            WithdrawCircuitInputs::new(merkle_tree, note, recipient, relayer, fee, refund)?;

        let (proof, _public_inputs) = self
            .prover
            .prove("tc", circuit_inputs.as_flat_map())
            .await?;

        let proof = proof_to_solidity_inputs(&proof);
        let call = Tornado::withdrawCall {
            _proof: proof,
            _root: circuit_inputs.merkle_root.into(),
            _nullifierHash: circuit_inputs.nullifier_hash.into(),
            _recipient: recipient,
            _relayer: relayer,
            _fee: fee,
            _refund: refund,
        };

        Ok(TxData {
            to: pool.address(),
            data: call.abi_encode(),
            value: refund,
        })
    }

    pub async fn sync(&mut self) -> Result<(), TornadoProviderError> {
        self.indexer.sync().await?;
        self.verify().await
    }

    pub async fn sync_to(&mut self, block: u64) -> Result<(), TornadoProviderError> {
        Ok(self.indexer.sync_to(block).await?)
    }

    pub async fn verify(&self) -> Result<(), TornadoProviderError> {
        Ok(self.indexer.verify().await?)
    }
}

fn proof_to_solidity_inputs(proof: &Proof) -> Bytes {
    let proof_elements: [U256; 8] = [
        proof.a.x,
        proof.a.y,
        //? Order of b elements are reversed to match Solidity's expected format
        proof.b.x[1],
        proof.b.x[0],
        proof.b.y[1],
        proof.b.y[0],
        proof.c.x,
        proof.c.y,
    ];
    let mut proof_bytes = Vec::with_capacity(256);
    for elem in &proof_elements {
        proof_bytes.extend_from_slice(&elem.to_be_bytes::<32>());
    }

    proof_bytes.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROOF_JSON: &str = r#"{
        "pi_a": [
                "13266136784835640332844746266198608263901891282482609564079887369169768624014",
                "17042632590340990663614784043794282016230679095846282033410052204483255659230"
        ],
        "pi_b": [
            [
                "10970198678781339136039451360739256402919493905733936018567807044072972302915",
                "17969804996632599314500752065264226621718741730732011051439003195120644879225"
            ],
            [
                "12838843182760738365092422718132994180261846015110376812162643571983566251328",
                "10274407733932184301684127680370353775282162047081888242499546519304733605"
            ]
        ],
        "pi_c": [
            "9457691057294082210004347434205523973500867149942472710321839541505714818518",
            "1969710731313679419138676630718164627777075664359407762059172130399473623983"
        ]
    }"#;

    #[test]
    fn test_proof_to_solidity_inputs() {
        let proof: Proof = serde_json::from_str(PROOF_JSON).unwrap();
        let solidity_inputs = proof_to_solidity_inputs(&proof);

        insta::assert_snapshot!(solidity_inputs);
    }
}
