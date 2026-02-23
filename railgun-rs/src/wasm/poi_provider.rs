use std::sync::Arc;

use alloy::{
    network::Ethereum,
    providers::{Provider, ProviderBuilder},
};
use wasm_bindgen::{JsError, prelude::wasm_bindgen};

use crate::{
    chain_config::get_chain_config,
    railgun::{
        PoiProvider, PoiProviderState, address::RailgunAddress, broadcaster::broadcaster::Fee,
        indexer::SubsquidSyncer, poi::PoiClient,
    },
    wasm::{
        JsFee, JsShieldBuilder,
        bindings::JsSigner,
        broadcaster::JsBroadcaster,
        indexer::{JsBalanceMap, JsSyncer},
        poi_transaction_builder::{JsPoiProvedTx, JsPoiTransactionBuilder},
        prover::JsProver,
    },
};

#[wasm_bindgen]
pub struct JsPoiProvider {
    inner: PoiProvider,
}

#[wasm_bindgen]
impl JsPoiProvider {
    pub async fn new(
        chain_id: u64,
        rpc_url: &str,
        utxo_syncer: JsSyncer,
        txid_subsquid_endpoint: &str,
        prover: JsProver,
    ) -> Result<JsPoiProvider, JsError> {
        let chain = get_chain_config(chain_id)
            .ok_or_else(|| JsError::new(&format!("Unsupported chain ID: {}", chain_id)))?;

        let provider = ProviderBuilder::new()
            .network::<Ethereum>()
            .connect(rpc_url)
            .await
            .unwrap()
            .erased();

        let utxo_syncer = Arc::from(utxo_syncer.take());
        let txid_syncer = Arc::new(SubsquidSyncer::new(txid_subsquid_endpoint));
        let prover = Arc::new(prover);

        let poi_url = chain.poi_endpoint.ok_or_else(|| {
            JsError::new(&format!(
                "Chain ID {} does not have a POI endpoint configured",
                chain_id
            ))
        })?;

        let poi_client = PoiClient::new(poi_url, chain_id)
            .await
            .map_err(|e| JsError::new(&format!("Failed to create POI client: {}", e)))?;

        Ok(JsPoiProvider {
            inner: PoiProvider::new(
                chain,
                provider,
                utxo_syncer,
                prover.clone(),
                txid_syncer,
                poi_client,
                prover,
            ),
        })
    }

    pub async fn from_state(
        state: &[u8],
        rpc_url: &str,
        utxo_syncer: JsSyncer,
        txid_subsquid_endpoint: &str,
        prover: JsProver,
    ) -> Result<JsPoiProvider, JsError> {
        let state: PoiProviderState = serde_json::from_slice(state)
            .map_err(|e| JsError::new(&format!("Failed to deserialize state: {}", e)))?;

        let chain_id = state.inner.chain_id;
        let chain = get_chain_config(chain_id)
            .ok_or_else(|| JsError::new(&format!("Unsupported chain ID: {}", chain_id)))?;

        let provider = ProviderBuilder::new()
            .network::<Ethereum>()
            .connect(rpc_url)
            .await
            .unwrap()
            .erased();

        let utxo_syncer = Arc::from(utxo_syncer.take());
        let txid_syncer = Arc::new(SubsquidSyncer::new(txid_subsquid_endpoint));
        let prover = Arc::new(prover);

        let poi_url = chain.poi_endpoint.ok_or_else(|| {
            JsError::new(&format!(
                "Chain ID {} does not have a POI endpoint configured",
                chain_id
            ))
        })?;

        let poi_client = PoiClient::new(poi_url, chain_id)
            .await
            .map_err(|e| JsError::new(&format!("Failed to create POI client: {}", e)))?;

        let inner = PoiProvider::from_state(
            state,
            provider,
            utxo_syncer,
            prover.clone(),
            txid_syncer,
            poi_client,
            prover,
        )
        .map_err(|e| JsError::new(&format!("Failed to create POI provider: {}", e)))?;

        Ok(JsPoiProvider { inner })
    }

    pub fn state(&self) -> Vec<u8> {
        let state = self.inner.state();
        serde_json::to_vec(&state).unwrap_or_default()
    }

    pub fn register(&mut self, signer: &JsSigner) {
        self.inner.register(signer.inner());
    }

    pub fn balance(&mut self, address: &str) -> Result<JsBalanceMap, JsError> {
        let address: RailgunAddress = address
            .parse()
            .map_err(|e| JsError::new(&format!("Invalid address: {}", e)))?;
        let balance = self.inner.balance(address);
        Ok(JsBalanceMap::new(balance))
    }

    pub fn shield(&self) -> JsShieldBuilder {
        self.inner.shield().into()
    }

    pub fn transact(&self) -> JsPoiTransactionBuilder {
        self.inner.transact().into()
    }

    pub async fn build(&self, builder: JsPoiTransactionBuilder) -> Result<JsPoiProvedTx, JsError> {
        let mut rng = rand::rng();
        let proved_tx = self
            .inner
            .build(builder.inner, &mut rng)
            .await
            .map_err(|e| JsError::new(&format!("Build error: {}", e)))?;

        Ok(proved_tx.into())
    }

    pub async fn build_broadcast(
        &mut self,
        builder: JsPoiTransactionBuilder,
        fee_payer: &JsSigner,
        fee: &JsFee,
    ) -> Result<JsPoiProvedTx, JsError> {
        let mut rng = rand::rng();
        let fee: Fee = fee.into();
        let proved_tx = self
            .inner
            .build_broadcast(builder.inner, fee_payer.inner(), &fee, &mut rng)
            .await
            .map_err(|e| JsError::new(&format!("Build/broadcast error: {}", e)))?;

        Ok(proved_tx.into())
    }

    pub async fn broadcast(
        &mut self,
        broadcaster: &JsBroadcaster,
        proved_tx: &JsPoiProvedTx,
    ) -> Result<(), JsError> {
        self.inner
            .broadcast(&broadcaster.inner, &proved_tx.inner)
            .await
            .map_err(|e| JsError::new(&format!("Broadcast error: {}", e)))
    }

    pub async fn await_indexed(&mut self, tx: &JsPoiProvedTx) -> Result<(), JsError> {
        self.inner
            .await_indexed(&tx.inner)
            .await
            .map_err(|e| JsError::new(&format!("Await indexed error: {}", e)))
    }

    pub async fn sync(&mut self) -> Result<(), JsError> {
        self.inner
            .sync()
            .await
            .map_err(|e| JsError::new(&format!("Sync error: {}", e)))
    }

    pub fn reset_indexer(&mut self) {
        self.inner.reset_indexer();
    }
}

impl JsPoiProvider {
    pub fn inner(&self) -> &PoiProvider {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut PoiProvider {
        &mut self.inner
    }
}
