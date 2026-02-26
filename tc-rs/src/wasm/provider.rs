use std::sync::Arc;

use alloy::primitives::Address;
use prover::Prover;
use rand::rng;
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

use crate::{
    indexer::{Indexer, RpcSyncer},
    note::Note,
    provider::{TornadoProvider, TornadoProviderState},
    tx_data::TxData,
    wasm::{
        JsProver,
        note::JsNote,
        pool::JsPool,
        syncer::{JsSyncer, new_dyn_provider},
        tx_data::JsTxData,
    },
};

#[wasm_bindgen]
pub struct JsTornadoProvider {
    inner: TornadoProvider,
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct JsDepositResult {
    tx_data: JsTxData,
    note: JsNote,
}

#[wasm_bindgen]
impl JsDepositResult {
    #[wasm_bindgen(getter, js_name = "txData")]
    pub fn tx_data(&self) -> JsTxData {
        self.tx_data.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn note(&self) -> JsNote {
        self.note.clone()
    }
}

#[wasm_bindgen]
impl JsTornadoProvider {
    /// Creates a new TornadoProvider
    ///
    /// @param pool Pool config (address, chain_id, etc.)
    /// @param rpc_url RPC URL used to create a Verifier (RpcSyncer) for on-chain root verification
    /// @param syncer Syncer used to index deposits/withdrawals
    /// @param prover Prover used to generate proofs
    pub async fn new(
        pool: &JsPool,
        rpc_url: &str,
        syncer: JsSyncer,
        prover: JsProver,
    ) -> Result<JsTornadoProvider, JsValue> {
        let provider = new_dyn_provider(rpc_url).await?;
        let verifier = Arc::new(RpcSyncer::new(provider, pool.inner.address()));
        let indexer = Indexer::new(syncer.inner(), verifier);
        let prover = Arc::new(prover);
        let inner = TornadoProvider::new(pool.chain_id(), indexer, prover);

        Ok(inner.into())
    }

    pub fn set_state(&mut self, state: &[u8]) -> Result<(), JsValue> {
        let state: TornadoProviderState = serde_json::from_slice(state)
            .map_err(|e| JsValue::from_str(&format!("Serde error: {}", e)))?;
        self.inner.set_state(state);
        Ok(())
    }

    pub fn state(&self) -> Result<Vec<u8>, JsValue> {
        let state = self.inner.state();
        serde_json::to_vec(&state).map_err(|e| JsValue::from_str(&format!("Serde error: {}", e)))
    }

    pub fn deposit(&self, pool: &JsPool) -> JsDepositResult {
        let mut rng = rng();
        let (tx_data, note) = self.inner.deposit(pool.inner.as_ref(), &mut rng);
        JsDepositResult {
            tx_data: tx_data.into(),
            note: note.into(),
        }
    }

    pub async fn withdraw(
        &self,
        pool: &JsPool,
        note: &JsNote,
        recipient: &str,
        relayer: &str,
        fee: js_sys::BigInt,
        refund: js_sys::BigInt,
    ) -> Result<JsTxData, JsValue> {
        let recipient: Address = recipient
            .parse()
            .map_err(|e| JsValue::from_str(&format!("Invalid recipient address: {}", e)))?;

        let relayer: Address = relayer
            .parse()
            .map_err(|e| JsValue::from_str(&format!("Invalid relayer address: {}", e)))?;

        let fee = bigint_to_u256(fee)?;
        let refund = bigint_to_u256(refund)?;

        let tx_data = self
            .inner
            .withdraw(
                pool.inner.as_ref(),
                &note.inner,
                recipient,
                relayer,
                fee,
                refund,
            )
            .await?;

        Ok(tx_data.into())
    }

    pub async fn sync(&mut self) -> Result<(), JsValue> {
        self.inner
            .sync()
            .await
            .map_err(|e| JsValue::from_str(&format!("Sync error: {}", e)))
    }

    pub async fn sync_to(&mut self, block: u64) -> Result<(), JsValue> {
        self.inner
            .sync_to(block)
            .await
            .map_err(|e| JsValue::from_str(&format!("Sync error: {}", e)))
    }
}

impl From<TornadoProvider> for JsTornadoProvider {
    fn from(inner: TornadoProvider) -> Self {
        Self { inner }
    }
}

fn bigint_to_u256(val: js_sys::BigInt) -> Result<ruint::aliases::U256, JsValue> {
    let s = val
        .to_string(10)
        .map_err(|e| JsValue::from_str(&format!("BigInt to string error: {:?}", e)))?
        .as_string()
        .ok_or_else(|| JsValue::from_str("BigInt to string returned non-string"))?;
    ruint::aliases::U256::from_str_radix(&s, 10)
        .map_err(|e| JsValue::from_str(&format!("U256 parse error: {}", e)))
}
