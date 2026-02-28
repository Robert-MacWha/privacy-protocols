use std::sync::Arc;

use eth_rpc::JsEthRpcAdapter;
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

use crate::broadcaster::{RelayerSyncer, RpcRelayerSyncer};

#[wasm_bindgen]
pub struct JsRelayerSyncer {
    inner: Arc<dyn RelayerSyncer>,
}

#[wasm_bindgen]
impl JsRelayerSyncer {
    /// Creates a new `JsRelayerSyncer` using an RPC URL.
    ///
    /// @param mainnet_provider RPC provider for a mainnet RPC provider.
    #[wasm_bindgen(js_name = "newRpc")]
    pub async fn new_rpc(mainnet_provider: JsEthRpcAdapter) -> Result<JsRelayerSyncer, JsValue> {
        Ok(RpcRelayerSyncer::new(Arc::new(mainnet_provider)).into())
    }
}

impl JsRelayerSyncer {
    pub fn inner(&self) -> Arc<dyn RelayerSyncer> {
        self.inner.clone()
    }
}

impl From<RpcRelayerSyncer> for JsRelayerSyncer {
    fn from(syncer: RpcRelayerSyncer) -> Self {
        JsRelayerSyncer {
            inner: Arc::new(syncer),
        }
    }
}
