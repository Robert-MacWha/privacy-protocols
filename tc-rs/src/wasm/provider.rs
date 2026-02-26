use std::sync::Arc;

use alloy::primitives::Address;
use rand::rng;
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

use crate::{
    provider::{PoolProviderState, TornadoProvider},
    wasm::{
        JsProver, note::JsNote, pool::JsPool, syncer::JsSyncer, tx_data::JsTxData,
        verifier::JsVerifier,
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
    /// @param syncer Syncer used to index deposits/withdrawals
    /// @param verifier Verifier used for on-chain root verification
    /// @param prover Prover used to generate proofs
    pub fn new(syncer: JsSyncer, verifier: JsVerifier, prover: JsProver) -> JsTornadoProvider {
        let inner = TornadoProvider::new(syncer.inner(), verifier.inner(), Arc::new(prover));
        inner.into()
    }

    pub fn add_pool(&mut self, pool: &JsPool) {
        self.inner.add_pool(pool.inner.clone());
    }

    pub fn add_pool_from_state(&mut self, state: &[u8]) -> Result<(), JsValue> {
        let state: PoolProviderState = serde_json::from_slice(state)
            .map_err(|e| JsValue::from_str(&format!("Serde error: {}", e)))?;
        self.inner.add_pool_from_state(state);
        Ok(())
    }

    pub fn state(&self) -> Result<Vec<u8>, JsValue> {
        let state = self.inner.state();
        serde_json::to_vec(&state).map_err(|e| JsValue::from_str(&format!("Serde error: {}", e)))
    }

    pub fn deposit(&self, pool: &JsPool) -> Result<JsDepositResult, JsValue> {
        let mut rng = rng();
        let (tx_data, note) = self.inner.deposit(&pool.inner, &mut rng)?;
        Ok(JsDepositResult {
            tx_data: tx_data.into(),
            note: note.into(),
        })
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
            .withdraw(&pool.inner, &note.inner, recipient, relayer, fee, refund)
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
