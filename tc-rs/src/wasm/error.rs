use wasm_bindgen::JsValue;

use crate::provider::TornadoProviderError;

impl From<TornadoProviderError> for JsValue {
    fn from(error: TornadoProviderError) -> Self {
        JsValue::from_str(&format!("TornadoProvider error: {}", error))
    }
}
