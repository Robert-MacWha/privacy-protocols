pub mod verifier;

mod merkle_proof;
mod merkle_tree;
mod smart_wallet_verifier;
mod utxo_tree;

pub use merkle_proof::{MerkleProof, MerkleRoot};
pub use merkle_tree::{
    MerkleTree, MerkleTreeError, MerkleTreeState, TOTAL_LEAVES, TREE_DEPTH,
    railgun_merkle_tree_zero,
};
pub use smart_wallet_verifier::SmartWalletUtxoVerifier;
pub use utxo_tree::{UtxoLeafHash, UtxoMerkleTree};
pub use verifier::{MerkleTreeVerifier, VerificationError};

#[cfg(feature = "poi")]
mod txid_tree;

#[cfg(feature = "poi")]
pub use txid_tree::{TxidLeafHash, TxidMerkleTree, UtxoTreeIndex};
