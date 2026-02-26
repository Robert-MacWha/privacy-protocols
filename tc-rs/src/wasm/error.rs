use wasm_bindgen::JsValue;

use crate::{indexer::IndexerError, provider::TornadoProviderError};

impl From<TornadoProviderError> for JsValue {
    fn from(error: TornadoProviderError) -> Self {
        JsValue::from_str(&format!("TornadoProvider error: {}", error))
    }
}
