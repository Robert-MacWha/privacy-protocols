use prover::{Proof, Prover, ProverError};
use ruint::aliases::U256;

pub async fn prove_transact(
    prover: &dyn Prover,
    inputs: &crate::circuit::inputs::TransactCircuitInputs,
) -> Result<(Proof, Vec<U256>), ProverError> {
    let nullifiers = inputs.nullifiers.len();
    let commitments = inputs.commitments_out.len();
    let circuit_name = format!("railgun/{:02}x{:02}", nullifiers, commitments);

    let proof = prover.prove(&circuit_name, inputs.as_flat_map()).await?;
    Ok(proof)
}

#[cfg(feature = "poi")]
pub async fn prove_poi(
    prover: &dyn Prover,
    inputs: &crate::circuit::inputs::PoiCircuitInputs,
) -> Result<(Proof, Vec<U256>), ProverError> {
    let nullifiers = inputs.nullifiers.len();
    let commitments = inputs.commitments.len();

    let circuit_name = format!("railgun/poi/{:02}x{:02}", nullifiers, commitments);
    let proof = prover.prove(&circuit_name, inputs.as_flat_map()).await?;
    Ok(proof)
}
