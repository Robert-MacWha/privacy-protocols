use std::collections::HashMap;

use alloy::{
    network::Ethereum,
    providers::{Provider, ProviderBuilder},
};
use wasm_bindgen::{JsError, prelude::wasm_bindgen};

use crate::{
    caip::AssetId,
    chain_config::get_chain_config,
    railgun::indexer::syncer::{ChainedSyncer, NoteSyncer, RpcSyncer, SubsquidSyncer},
};

#[wasm_bindgen]
pub struct JsSyncer {
    inner: Option<Box<dyn NoteSyncer>>,
}

#[wasm_bindgen]
pub struct JsBalanceMap {
    inner: HashMap<AssetId, u128>,
}

#[wasm_bindgen]
impl JsSyncer {
    #[wasm_bindgen(js_name = "withSubsquid")]
    pub fn with_subsquid(endpoint: &str) -> JsSyncer {
        JsSyncer {
            inner: Some(Box::new(SubsquidSyncer::new(endpoint))),
        }
    }

    #[wasm_bindgen(js_name = "withRpc")]
    pub async fn with_rpc(
        rpc_url: &str,
        chain_id: u64,
        batch_size: u64,
    ) -> Result<JsSyncer, JsError> {
        let provider = ProviderBuilder::new()
            .network::<Ethereum>()
            .connect(rpc_url)
            .await
            .unwrap()
            .erased();

        let chain = get_chain_config(chain_id)
            .ok_or_else(|| JsError::new(&format!("Unsupported chain ID: {}", chain_id)))?;

        Ok(JsSyncer {
            inner: Some(Box::new(
                RpcSyncer::new(provider, chain).with_batch_size(batch_size),
            )),
        })
    }

    #[wasm_bindgen(js_name = "withChained")]
    pub fn with_chained(syncers: Vec<JsSyncer>) -> JsSyncer {
        let inner: Vec<Box<dyn NoteSyncer>> = syncers
            .into_iter()
            .filter_map(|mut js_syncer| js_syncer.inner.take())
            .collect();
        JsSyncer {
            inner: Some(Box::new(ChainedSyncer::new(inner))),
        }
    }
}

impl JsSyncer {
    /// Takes ownership of the inner syncer, leaving None behind.
    pub fn take(mut self) -> Box<dyn NoteSyncer> {
        self.inner.take().expect("JsSyncer already consumed")
    }
}

#[wasm_bindgen]
impl JsBalanceMap {
    pub fn get(&self, asset_id: &str) -> Option<js_sys::BigInt> {
        let asset_id: AssetId = asset_id.parse().ok()?;
        self.inner
            .get(&asset_id)
            .map(|balance| js_sys::BigInt::from(*balance))
    }

    pub fn keys(&self) -> Vec<String> {
        self.inner
            .keys()
            .map(|asset_id| asset_id.to_string())
            .collect()
    }
}

impl JsBalanceMap {
    pub fn new(inner: HashMap<AssetId, u128>) -> Self {
        Self { inner }
    }
}
