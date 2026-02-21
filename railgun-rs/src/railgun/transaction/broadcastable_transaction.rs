use alloy::primitives::TxHash;
use rand::Rng;

use crate::railgun::{
    broadcaster::broadcaster::{BroadcastError, Broadcaster},
    poi::PendingPoiSubmitter,
    transaction::PoiProvedTx,
};

/// A transaction that is ready to be broadcast through a broadcaster.
pub struct BroadcastableTx<'a> {
    inner: PoiProvedTx,
    broadcaster: Broadcaster,
    submitter: &'a mut PendingPoiSubmitter,
}

impl<'a> BroadcastableTx<'a> {
    pub fn new(
        inner: PoiProvedTx,
        broadcaster: Broadcaster,
        submitter: &'a mut PendingPoiSubmitter,
    ) -> Self {
        Self {
            inner,
            broadcaster,
            submitter,
        }
    }

    pub async fn broadcast<R: Rng>(self, rng: &mut R) -> Result<TxHash, BroadcastError> {
        let txhash = self.broadcaster.broadcast(&self.inner, rng).await?;

        for op in &self.inner.operations {
            self.submitter.register(op);
        }

        Ok(txhash)
    }
}
