use alloy::primitives::Address;
use wasm_bindgen::{JsError, prelude::wasm_bindgen};

use crate::{
    caip::AssetId,
    railgun::{
        address::RailgunAddress,
        transaction::{PoiProvedTx, PoiTransactionBuilder},
    },
    wasm::bindings::JsSigner,
};

/// Builder for POI transact transactions (transfers and unshields).
#[wasm_bindgen]
pub struct JsPoiTransactionBuilder {
    pub(crate) inner: PoiTransactionBuilder,
}

#[wasm_bindgen]
pub struct JsPoiProvedTx {
    pub(crate) inner: PoiProvedTx,
}

#[wasm_bindgen]
impl JsPoiTransactionBuilder {
    /// Add a transfer operation.
    ///
    /// - `from`: Signer for the sender
    /// - `to`: Railgun address (0zk...)
    /// - `asset`: Asset ID (e.g., "erc20:0x...")
    /// - `value`: Amount as decimal string
    /// - `memo`: Memo string
    pub fn transfer(
        self,
        from: &JsSigner,
        to: &str,
        asset: &str,
        value: &str,
        memo: &str,
    ) -> Result<Self, JsError> {
        let to: RailgunAddress = to
            .parse()
            .map_err(|e| JsError::new(&format!("Invalid recipient address: {}", e)))?;

        let asset: AssetId = asset
            .parse()
            .map_err(|e| JsError::new(&format!("Invalid asset ID: {}", e)))?;

        let value: u128 = value
            .parse()
            .map_err(|e| JsError::new(&format!("Invalid amount: {}", e)))?;

        Ok(self
            .inner
            .transfer(from.inner(), to, asset, value, memo)
            .into())
    }

    /// Add an unshield operation.
    ///
    /// - `from`: Signer for the sender
    /// - `to`: Ethereum address (0x...)
    /// - `asset`: Asset ID (e.g., "erc20:0x...")
    /// - `value`: Amount as decimal string
    pub fn unshield(
        self,
        from: &JsSigner,
        to: &str,
        asset: &str,
        value: &str,
    ) -> Result<Self, JsError> {
        let to: Address = to
            .parse()
            .map_err(|e| JsError::new(&format!("Invalid recipient address: {}", e)))?;

        let asset: AssetId = asset
            .parse()
            .map_err(|e| JsError::new(&format!("Invalid asset ID: {}", e)))?;

        let value: u128 = value
            .parse()
            .map_err(|e| JsError::new(&format!("Invalid amount: {}", e)))?;

        Ok(self
            .inner
            .set_unshield(from.inner(), to, asset, value)
            .into())
    }
}

impl From<PoiTransactionBuilder> for JsPoiTransactionBuilder {
    fn from(inner: PoiTransactionBuilder) -> Self {
        Self { inner }
    }
}

impl From<PoiProvedTx> for JsPoiProvedTx {
    fn from(inner: PoiProvedTx) -> Self {
        Self { inner }
    }
}
