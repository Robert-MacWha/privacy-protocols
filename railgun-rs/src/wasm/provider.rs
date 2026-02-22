use std::sync::Arc;

use alloy::{
    network::Ethereum,
    providers::{DynProvider, Provider, ProviderBuilder},
};
use wasm_bindgen::{JsError, prelude::wasm_bindgen};

use crate::{
    chain_config::get_chain_config,
    railgun::{RailgunProvider, RailgunProviderState, address::RailgunAddress},
    wasm::{
        JsShieldBuilder, JsSigner, JsTransactionBuilder, JsTxData,
        indexer::{JsBalanceMap, JsSyncer},
        prover::JsProver,
    },
};

#[wasm_bindgen]
pub struct JsRailgunProvider {
    inner: RailgunProvider,
}

#[wasm_bindgen]
impl JsRailgunProvider {
    pub async fn new(
        chain_id: u64,
        rpc_url: &str,
        syncer: JsSyncer,
        prover: JsProver,
    ) -> Result<JsRailgunProvider, JsError> {
        let chain = get_chain_config(chain_id)
            .ok_or_else(|| JsError::new(&format!("Unsupported chain ID: {}", chain_id)))?;

        let provider = build_provider(rpc_url).await;
        let syncer = Arc::from(syncer.take());
        let prover: Arc<JsProver> = Arc::new(prover);

        Ok(JsRailgunProvider {
            inner: RailgunProvider::new(chain, provider, syncer, prover),
        })
    }

    pub async fn from_state(
        state: &[u8],
        rpc_url: &str,
        syncer: JsSyncer,
        prover: JsProver,
    ) -> Result<JsRailgunProvider, JsError> {
        let state: RailgunProviderState = serde_json::from_slice(state)
            .map_err(|e| JsError::new(&format!("Failed to deserialize state: {}", e)))?;

        let provider = build_provider(rpc_url).await;
        let syncer = Arc::from(syncer.take());
        let prover: Arc<JsProver> = Arc::new(prover);

        let inner = RailgunProvider::from_state(state, provider, syncer, prover)
            .map_err(|e| JsError::new(&format!("Failed to create provider: {}", e)))?;

        Ok(JsRailgunProvider { inner })
    }

    pub fn register(&mut self, signer: &JsSigner) {
        self.inner.register(signer.inner());
    }

    pub async fn sync(&mut self) -> Result<(), JsError> {
        self.inner
            .sync()
            .await
            .map_err(|e| JsError::new(&format!("Sync error: {}", e)))
    }

    pub fn balance(&mut self, address: &str) -> Result<JsBalanceMap, JsError> {
        let address: RailgunAddress = address
            .parse()
            .map_err(|e| JsError::new(&format!("Invalid address: {}", e)))?;
        let balance = self.inner.balance(address);
        Ok(JsBalanceMap::new(balance))
    }

    pub fn export_state(&self) -> Vec<u8> {
        let state = self.inner.state();
        serde_json::to_vec(&state).unwrap_or_default()
    }

    pub fn shield(&self) -> JsShieldBuilder {
        self.inner.shield().into()
    }

    pub fn transact(&self) -> JsTransactionBuilder {
        self.inner.transact().into()
    }

    pub async fn build(&self, builder: JsTransactionBuilder) -> Result<JsTxData, JsError> {
        let mut rng = rand::rng();
        let proved_tx = self
            .inner
            .build(builder.into(), &mut rng)
            .await
            .map_err(|e| JsError::new(&format!("Build error: {}", e)))?;

        Ok(proved_tx.tx_data.into())
    }

    pub fn reset_indexer(&mut self) {
        self.inner.reset_indexer();
    }
}

impl JsRailgunProvider {
    pub fn inner(&self) -> &RailgunProvider {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut RailgunProvider {
        &mut self.inner
    }
}

async fn build_provider(rpc_url: &str) -> DynProvider {
    ProviderBuilder::new()
        .network::<Ethereum>()
        .connect(rpc_url)
        .await
        .unwrap()
        .erased()
}
