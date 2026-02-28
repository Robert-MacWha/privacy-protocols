use reqwest::StatusCode;
use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;
use tracing::{info, warn};

#[cfg(feature = "poi")]
use crate::railgun::indexer::TransactionSyncer;
use crate::railgun::indexer::{
    NoteSyncer,
    syncer::{self, subsquid_types::*, syncer::SyncerError},
};

pub struct SubsquidSyncer {
    client: reqwest::Client,
    url: String,
    batch_size: u64,
    max_retries: usize,
    retry_delay: web_time::Duration,
}

#[derive(Debug, Error)]
pub enum SubsquidSyncerError {
    #[error("Serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Request failed with status {0}: {1}")]
    Request(StatusCode, String),
    #[error("GraphQL error: {0}")]
    GraphQL(String),
}

const COMMITMENTS_QUERY: &str = include_str!("./subsquid_graphql/commitments.graphql");
const NULLIFIERS_QUERY: &str = include_str!("./subsquid_graphql/nullifiers.graphql");
#[cfg(feature = "poi")]
const OPERATIONS_QUERY: &str = include_str!("./subsquid_graphql/operations.graphql");
const BLOCK_NUMBER_QUERY: &str = include_str!("./subsquid_graphql/block_number.graphql");

impl SubsquidSyncer {
    pub fn new(url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            url: url.to_string(),
            batch_size: 20000,
            max_retries: 3,
            retry_delay: web_time::Duration::from_secs(1),
        }
    }

    pub fn with_batch_size(mut self, batch_size: u64) -> Self {
        self.batch_size = batch_size;
        self
    }

    pub fn with_retry_policy(
        mut self,
        max_retries: usize,
        retry_delay: web_time::Duration,
    ) -> Self {
        self.max_retries = max_retries;
        self.retry_delay = retry_delay;
        self
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl NoteSyncer for SubsquidSyncer {
    async fn latest_block(&self) -> Result<u64, SyncerError> {
        Ok(self.latest_block().await?)
    }

    async fn sync(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<syncer::SyncEvent>, SyncerError> {
        info!("Starting Subsquid sync {}-{}", from_block, to_block);

        let mut events = Vec::new();

        let mut commitments = self.commitments(from_block, to_block).await?;
        events.append(&mut commitments);

        let mut nullifiers = self.nullifiers(from_block, to_block).await?;
        events.append(&mut nullifiers);

        Ok(events)
    }
}

#[cfg(feature = "poi")]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl TransactionSyncer for SubsquidSyncer {
    async fn latest_block(&self) -> Result<u64, SyncerError> {
        Ok(self.latest_block().await?)
    }

    async fn sync(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<syncer::Operation>, SyncerError> {
        let operations = self.operations(from_block, to_block).await?;
        Ok(operations)
    }
}

impl SubsquidSyncer {
    async fn latest_block(&self) -> Result<u64, SubsquidSyncerError> {
        let data: BlockNumberResponsese = self.post_graphql_retry(BLOCK_NUMBER_QUERY, ()).await?;
        let latest_block = data
            .transactions
            .first()
            .map(|tx| tx.block_number)
            .unwrap_or(0);
        Ok(latest_block)
    }

    async fn commitments(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<syncer::SyncEvent>, SubsquidSyncerError> {
        let mut id_gt = String::new();

        let mut all_commitments = Vec::new();
        loop {
            let vars = QueryVars {
                id_gt,
                block_number_gte: from_block,
                block_number_lte: to_block,
                limit: self.batch_size,
            };
            let data: CommitmentsResponse =
                self.post_graphql_retry(COMMITMENTS_QUERY, vars).await?;
            if data.commitments.is_empty() {
                break;
            }

            id_gt = data
                .commitments
                .last()
                .map(|c| c.id.clone())
                .unwrap_or_default();
            let latest_block = data.commitments.last().map(|c| c.block_number).unwrap_or(0);
            let commitments: Vec<syncer::SyncEvent> = data
                .commitments
                .into_iter()
                .map(syncer::SyncEvent::from)
                .collect();

            all_commitments.extend(commitments);
            info!(
                "{}/{} ({} commitments)",
                latest_block,
                to_block,
                all_commitments.len()
            );
        }

        Ok(all_commitments)
    }

    async fn nullifiers(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<syncer::SyncEvent>, SubsquidSyncerError> {
        let mut id_gt = String::new();

        let mut all_nullifiers = Vec::new();
        loop {
            let vars = QueryVars {
                id_gt,
                block_number_gte: from_block,
                block_number_lte: to_block,
                limit: self.batch_size,
            };
            let data: NullifiersResponse = self.post_graphql_retry(NULLIFIERS_QUERY, vars).await?;
            if data.nullifiers.is_empty() {
                break;
            }

            id_gt = data
                .nullifiers
                .last()
                .map(|c| c.id.clone())
                .unwrap_or_default();
            let latest_block = data.nullifiers.last().map(|n| n.block_number).unwrap_or(0);
            let nullifiers: Vec<syncer::SyncEvent> = data
                .nullifiers
                .into_iter()
                .map(syncer::SyncEvent::from)
                .collect();

            all_nullifiers.extend(nullifiers);
            info!(
                "{}/{} ({} nullifiers)",
                latest_block,
                to_block,
                all_nullifiers.len()
            );
        }

        Ok(all_nullifiers)
    }

    #[cfg(feature = "poi")]
    async fn operations(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<syncer::Operation>, SubsquidSyncerError> {
        let mut id_gt = String::new();

        let mut all_operations = Vec::new();
        loop {
            let vars = QueryVars {
                id_gt,
                block_number_gte: from_block,
                block_number_lte: to_block,
                limit: self.batch_size,
            };
            let data: OperationsResponse = self.post_graphql_retry(OPERATIONS_QUERY, vars).await?;
            if data.operations.is_empty() {
                break;
            }

            id_gt = data
                .operations
                .last()
                .map(|c| c.id.clone())
                .unwrap_or_default();
            let latest_block = data
                .operations
                .last()
                .map(|op| op.block_number)
                .unwrap_or(0);
            let operations: Vec<syncer::Operation> = data
                .operations
                .into_iter()
                .map(syncer::Operation::from)
                .collect();

            all_operations.extend(operations);
            info!(
                "{}/{} ({} operations)",
                latest_block,
                to_block,
                all_operations.len()
            );
        }

        Ok(all_operations)
    }

    async fn post_graphql_retry<V: Serialize, R: DeserializeOwned>(
        &self,
        query: &'static str,
        variables: V,
    ) -> Result<R, SubsquidSyncerError> {
        let body = GraphqlRequest { query, variables };
        let json_body = serde_json::to_vec(&body)?;

        let mut attempt = 0;
        loop {
            match self.post_graphql(json_body.clone()).await {
                Ok(data) => return Ok(data),
                Err(e) => {
                    attempt += 1;
                    if attempt > self.max_retries {
                        return Err(e);
                    }

                    warn!(
                        "GraphQL request failed (attempt {}/{}): {}",
                        attempt, self.max_retries, e
                    );
                    common::sleep(self.retry_delay).await;
                }
            }
        }
    }

    async fn post_graphql<R: DeserializeOwned>(
        &self,
        body: Vec<u8>,
    ) -> Result<R, SubsquidSyncerError> {
        let resp = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(SubsquidSyncerError::Request(
                resp.status(),
                resp.text().await.unwrap_or_default(),
            ));
        }

        let value: serde_json::Value = resp.json().await?;
        // info!("Deserializing: {}", &value);
        let graphql_resp: GraphqlResponse<R> = serde_json::from_value(value)?;
        if let Some(errors) = graphql_resp.errors {
            return Err(SubsquidSyncerError::GraphQL(
                errors
                    .into_iter()
                    .map(|e| e.message)
                    .collect::<Vec<_>>()
                    .join("; "),
            ));
        }

        let Some(data) = graphql_resp.data else {
            return Err(SubsquidSyncerError::GraphQL(
                "No data in response".to_string(),
            ));
        };

        Ok(data)
    }
}

impl From<SubsquidSyncerError> for SyncerError {
    fn from(e: SubsquidSyncerError) -> Self {
        SyncerError::Syncer(Box::new(e))
    }
}
