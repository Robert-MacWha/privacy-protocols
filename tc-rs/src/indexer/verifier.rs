use crypto::merkle_tree::MerkleRoot;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VerifierError {
    #[error("Invalid root: {root:?}")]
    InvalidRoot { root: MerkleRoot },
    #[error("Other error: {0}")]
    Other(Box<dyn std::error::Error + Send + Sync>),
}

#[cfg_attr(not(feature = "wasm"), async_trait::async_trait)]
#[cfg_attr(feature = "wasm", async_trait::async_trait(?Send))]
pub trait Verifier: Send + Sync {
    async fn verify(&self, root: MerkleRoot) -> Result<(), VerifierError>;
}
