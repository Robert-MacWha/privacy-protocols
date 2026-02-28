pub mod inputs;
pub mod prover;
pub mod witness;

#[cfg(not(target_arch = "wasm32"))]
pub mod artifact_loader;
#[cfg(not(target_arch = "wasm32"))]
pub mod native;
