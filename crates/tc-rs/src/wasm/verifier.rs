use std::sync::Arc;

use eth_rpc::JsEthRpcAdapter;
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

use crate::indexer::{RpcSyncer, Verifier};

#[wasm_bindgen]
pub struct JsVerifier {
    inner: Arc<dyn Verifier>,
}

#[wasm_bindgen]
impl JsVerifier {
    #[wasm_bindgen(js_name = "newRpc")]
    pub async fn new_rpc(provider: JsEthRpcAdapter) -> Result<JsVerifier, JsValue> {
        Ok(RpcSyncer::new(Arc::new(provider)).into())
    }
}

impl JsVerifier {
    pub fn inner(&self) -> Arc<dyn Verifier> {
        self.inner.clone()
    }
}

impl From<RpcSyncer> for JsVerifier {
    fn from(syncer: RpcSyncer) -> Self {
        JsVerifier {
            inner: Arc::new(syncer),
        }
    }
}
