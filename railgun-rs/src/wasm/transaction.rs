use alloy::primitives::Address;
use wasm_bindgen::{JsError, prelude::wasm_bindgen};

use crate::{
    caip::AssetId,
    railgun::{
        address::RailgunAddress,
        transaction::{PoiProvedTx, TransactionBuilder},
    },
    wasm::bindings::JsSigner,
};

/// Builder for transact transactions (transfers and unshields).
///
/// Stores transfer/unshield data, then borrows the provider only during `build()`.
///
/// @example
/// ```typescript
/// const builder = new JsTransactionBuilder();
/// builder.transfer(signer, "0zk...", wasm.erc20_asset("0x..."), "100", "memo");
/// const txData = await builder.build(provider);
/// ```
#[wasm_bindgen]
pub struct JsTransactionBuilder {
    inner: TransactionBuilder,
}

#[wasm_bindgen]
pub struct JsPoiProvedTx {
    inner: PoiProvedTx,
}

#[wasm_bindgen]
impl JsTransactionBuilder {
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

        Ok(Self {
            inner: self.inner.transfer(from.inner(), to, asset, value, memo),
        })
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

        Ok(Self {
            inner: self.inner.set_unshield(from.inner(), to, asset, value),
        })
    }
}

impl From<TransactionBuilder> for JsTransactionBuilder {
    fn from(inner: TransactionBuilder) -> Self {
        Self { inner }
    }
}

impl From<JsTransactionBuilder> for TransactionBuilder {
    fn from(builder: JsTransactionBuilder) -> Self {
        builder.inner
    }
}

impl From<PoiProvedTx> for JsPoiProvedTx {
    fn from(inner: PoiProvedTx) -> Self {
        Self { inner }
    }
}

impl From<JsPoiProvedTx> for PoiProvedTx {
    fn from(proved: JsPoiProvedTx) -> Self {
        proved.inner
    }
}
