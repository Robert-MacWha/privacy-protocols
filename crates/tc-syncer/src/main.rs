use std::{
    fs,
    io::Write as _,
    path::{Path, PathBuf},
    sync::Arc,
};

use alloy::{network::Ethereum, providers::ProviderBuilder};
use clap::Parser;
use serde::Deserialize;
use tc_rs::{
    POOLS,
    indexer::{RpcSyncer, Syncer},
};
use tracing::info;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "./sync-input")]
    input: PathBuf,
    #[arg(long, default_value = "./sync-output")]
    output: PathBuf,
}

#[derive(Deserialize)]
struct BlockNumberOnly {
    block_number: u64,
}

fn last_block_number(path: &Path) -> Option<u64> {
    let content = fs::read_to_string(path).ok()?;
    content
        .lines()
        .filter(|l| !l.is_empty())
        .last()
        .and_then(|l| serde_json::from_str::<BlockNumberOnly>(l).ok())
        .map(|e| e.block_number)
}

fn rpc_url_for_chain(chain_id: u64) -> Option<String> {
    let var_name = match chain_id {
        1 => "RPC_URL_MAINNET",
        11155111 => "RPC_URL_SEPOLIA",
        10 => "RPC_URL_OPTIMISM",
        _ => return None,
    };
    std::env::var(var_name).ok()
}

fn append_to_output(input: &Path, output: &Path, lines: &str) -> std::io::Result<()> {
    if input != output && input.exists() {
        fs::copy(input, output)?;
    }
    if lines.is_empty() {
        return Ok(());
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(output)?;
    file.write_all(lines.as_bytes())?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    fs::create_dir_all(&args.output)?;

    for pool in POOLS {
        let Some(rpc_url) = rpc_url_for_chain(pool.chain_id) else {
            info!("Skipping pool {} (no RPC URL configured)", pool);
            continue;
        };

        let prefix = format!("{}_{}_{}", pool.chain_id, pool.symbol(), pool.amount());
        let deposits_in = args.input.join(format!("{prefix}_deposits.ndjson"));
        let nullifiers_in = args.input.join(format!("{prefix}_nullifiers.ndjson"));
        let deposits_out = args.output.join(format!("{prefix}_deposits.ndjson"));
        let nullifiers_out = args.output.join(format!("{prefix}_nullifiers.ndjson"));

        let from_block = [
            last_block_number(&deposits_in),
            last_block_number(&nullifiers_in),
        ]
        .into_iter()
        .flatten()
        .max()
        .map(|b| b + 1)
        .unwrap_or(0);

        let from_block = from_block.max(pool.deployed_block);
        let provider = ProviderBuilder::new()
            .network::<Ethereum>()
            .connect(&rpc_url)
            .await?;
        let rpc_syncer = RpcSyncer::new(Arc::new(provider)).with_batch_size(100_000);

        let latest_block = rpc_syncer.latest_block().await?;
        info!("{prefix}: from_block={from_block}, latest={latest_block}");

        if from_block > latest_block {
            info!("{prefix}: already up to date");
            continue;
        }

        let commitments = rpc_syncer
            .sync_commitments(pool.address, from_block, latest_block)
            .await?;
        let nullifiers = rpc_syncer
            .sync_nullifiers(pool.address, from_block, latest_block)
            .await?;

        info!(
            "{prefix}: {} deposits, {} withdrawals",
            commitments.len(),
            nullifiers.len()
        );

        let mut deposit_lines = String::new();
        for c in &commitments {
            deposit_lines.push_str(&serde_json::to_string(c)?);
            deposit_lines.push('\n');
        }

        let mut nullifier_lines = String::new();
        for n in &nullifiers {
            nullifier_lines.push_str(&serde_json::to_string(n)?);
            nullifier_lines.push('\n');
        }

        append_to_output(&deposits_in, &deposits_out, &deposit_lines)?;
        append_to_output(&nullifiers_in, &nullifiers_out, &nullifier_lines)?;
    }

    info!("Done");
    Ok(())
}
