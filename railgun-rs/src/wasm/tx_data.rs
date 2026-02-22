use wasm_bindgen::prelude::wasm_bindgen;

use crate::railgun::transaction::TxData;

/// Transaction data output for EVM submission
#[wasm_bindgen]
pub struct JsTxData {
    inner: TxData,
}

#[wasm_bindgen]
impl JsTxData {
    /// Contract address to send the transaction to (checksummed 0x...)
    #[wasm_bindgen(getter)]
    pub fn to(&self) -> String {
        self.inner.to.to_checksum(None)
    }

    /// Raw calldata bytes
    #[wasm_bindgen(getter)]
    pub fn data(&self) -> Vec<u8> {
        self.inner.data.clone()
    }

    /// ETH value to send (decimal string, usually "0")
    #[wasm_bindgen(getter)]
    pub fn value(&self) -> String {
        self.inner.value.to_string()
    }

    /// Returns 0x-prefixed hex-encoded calldata
    #[wasm_bindgen(getter, js_name = "dataHex")]
    pub fn data_hex(&self) -> String {
        format!("0x{}", hex::encode(&self.inner.data))
    }
}

impl From<TxData> for JsTxData {
    fn from(tx_data: TxData) -> Self {
        JsTxData { inner: tx_data }
    }
}
