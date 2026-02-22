mod bindings;
mod broadcaster;
mod fee;
mod indexer;
mod poi_client;
mod poi_provider;
mod prover;
mod provider;
mod shield_builder;
mod transaction;
mod tx_data;

pub use bindings::{JsChainConfig, JsSigner, erc20_asset, get_chain_config, init_panic_hook};
pub use broadcaster::JsBroadcasterManager;
pub use fee::JsFee;
pub use indexer::JsSyncer;
pub use poi_provider::JsPoiProvider;
pub use prover::{JsProofResponse, JsProver};
pub use provider::JsRailgunProvider;
pub use shield_builder::JsShieldBuilder;
pub use transaction::JsTransactionBuilder;
pub use tx_data::JsTxData;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(start)]
pub fn wasm_start() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default_with_config(
        tracing_wasm::WASMLayerConfigBuilder::new()
            .set_max_level(tracing::Level::INFO)
            .build(),
    );
}
