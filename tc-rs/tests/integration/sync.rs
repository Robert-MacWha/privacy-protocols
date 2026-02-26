use std::{path::Path, sync::Arc};

use alloy::{
    network::Ethereum,
    primitives::{Address, address},
    providers::{Provider, ProviderBuilder},
};
use tc_rs::indexer::{CacheSyncer, ChainedSyncer, Indexer, RpcSyncer};
use tracing::info;

const TORNADO_ADDRESS: Address = address!("0x8cc930096b4df705a007c4a039bdfa1320ed2508");

#[tokio::test]
#[ignore]
async fn test_sync() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_test_writer()
        .try_init()
        .ok();

    let deposits_path = Path::new("./tests/fixtures/deposits_eth_1.json");
    let withdrawals_path = Path::new("./tests/fixtures/withdrawals_eth_1.json");
    let cache_syncer = Arc::new(CacheSyncer::from_files(deposits_path, withdrawals_path).unwrap());

    // let rpc_url = std::env::var("FORK_URL_SEPOLIA").expect("FORK_URL_SEPOLIA must be set");
    let rpc_url = "http://localhost:8545";
    let provider = ProviderBuilder::new()
        .network::<Ethereum>()
        .connect(&rpc_url)
        .await
        .unwrap()
        .erased();

    let rpc_syncer = Arc::new(RpcSyncer::new(provider, TORNADO_ADDRESS).with_batch_size(10000));
    let syncer: Arc<ChainedSyncer> =
        Arc::new(ChainedSyncer::new(vec![cache_syncer, rpc_syncer.clone()]));
    let mut indexer = Indexer::new(syncer.clone(), rpc_syncer.clone());

    info!("Syncing indexer...");
    indexer.sync().await.unwrap();

    info!("Verifying computed root against on-chain root...");
    assert!(
        indexer.verify().await.is_ok(),
        "computed root should be known on-chain"
    );
}
