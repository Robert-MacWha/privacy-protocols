use wasm_bindgen::{JsError, prelude::wasm_bindgen};

use crate::{
    caip::AssetId,
    railgun::{address::RailgunAddress, transaction::ShieldBuilder},
    wasm::tx_data::JsTxData,
};

/// Builder for shield transactions (self-broadcast only, no prover needed)
#[wasm_bindgen]
pub struct JsShieldBuilder {
    inner: ShieldBuilder,
}

#[wasm_bindgen]
impl JsShieldBuilder {
    /// Add a shield operation.
    ///
    /// - `recipient`: Railgun address (0zk...)
    /// - `asset`: Asset ID (e.g., "erc20:0x...")
    /// - `amount`: Amount as decimal string
    pub fn shield(self, recipient: &str, asset: &str, amount: &str) -> Result<Self, JsError> {
        let recipient: RailgunAddress = recipient
            .parse()
            .map_err(|e| JsError::new(&format!("Invalid recipient address: {}", e)))?;

        let asset: AssetId = asset
            .parse()
            .map_err(|e| JsError::new(&format!("Invalid asset ID: {}", e)))?;

        let amount: u128 = amount
            .parse()
            .map_err(|e| JsError::new(&format!("Invalid amount: {}", e)))?;

        Ok(JsShieldBuilder {
            inner: self.inner.shield(recipient, asset, amount),
        })
    }

    /// Build the shield transaction calldata
    pub fn build(self) -> Result<JsTxData, JsError> {
        let tx = self
            .inner
            .build()
            .map_err(|e| JsError::new(&format!("Shield build error: {}", e)))?;

        Ok(tx.into())
    }
}

impl From<ShieldBuilder> for JsShieldBuilder {
    fn from(inner: ShieldBuilder) -> Self {
        Self { inner }
    }
}
