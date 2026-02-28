use ruint::aliases::U256;

#[cfg(feature = "poi")]
use crate::circuit::inputs::PoiCircuitInputs;
use crate::circuit::inputs::TransactCircuitInputs;

pub type PublicInputs = Vec<U256>;

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait TransactProver: common::MaybeSend {
    async fn prove_transact(
        &self,
        inputs: &TransactCircuitInputs,
    ) -> Result<(prover::Proof, PublicInputs), Box<dyn std::error::Error>>;
}

#[cfg(feature = "poi")]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait PoiProver: common::MaybeSend {
    async fn prove_poi(
        &self,
        inputs: &PoiCircuitInputs,
    ) -> Result<(prover::Proof, PublicInputs), Box<dyn std::error::Error>>;
}

#[cfg(feature = "poi")]
pub trait Prover: TransactProver + PoiProver {}
#[cfg(feature = "poi")]
impl<T: TransactProver + PoiProver> Prover for T {}
