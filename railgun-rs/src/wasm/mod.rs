mod bindings;
mod indexer;
mod prover;
mod provider;
mod shield_builder;
mod transaction_builder;
mod tx_data;

pub use bindings::{JsChainConfig, JsSigner, erc20_asset, get_chain_config, init_panic_hook};
pub use indexer::JsSyncer;
pub use prover::{JsProofResponse, JsProver};
pub use provider::JsRailgunProvider;
pub use shield_builder::JsShieldBuilder;
pub use transaction_builder::JsTransactionBuilder;
pub use tx_data::JsTxData;
use wasm_bindgen::prelude::wasm_bindgen;

#[cfg(feature = "poi")]
mod broadcaster;
#[cfg(feature = "poi")]
mod fee;
#[cfg(feature = "poi")]
mod poi_client;
#[cfg(feature = "poi")]
mod poi_provider;
#[cfg(feature = "poi")]
mod poi_transaction_builder;

#[cfg(feature = "poi")]
pub use broadcaster::JsBroadcasterManager;
#[cfg(feature = "poi")]
pub use fee::JsFee;
#[cfg(feature = "poi")]
pub use poi_provider::JsPoiProvider;
#[cfg(feature = "poi")]
pub use poi_transaction_builder::{JsPoiProvedTx, JsPoiTransactionBuilder};

#[wasm_bindgen(start)]
pub fn wasm_start() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default_with_config(
        tracing_wasm::WASMLayerConfigBuilder::new()
            .set_max_level(tracing::Level::INFO)
            .build(),
    );
}
