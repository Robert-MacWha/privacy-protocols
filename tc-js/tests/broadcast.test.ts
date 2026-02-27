import { createPublicClient, createWalletClient, http } from "viem";
import { test } from "vitest";
import { createProver } from "../src/prover-adapter.js";
import { sepolia } from "viem/chains";
import { privateKeyToAccount } from "viem/accounts";
import { JsBroadcastProvider, JsPool, JsRelayerSyncer, JsSyncer, JsVerifier } from "../src/pkg/tc_rs.js";
import { readFileSync } from "node:fs";

const RPC_URL_MAINNET = process.env.FORK_URL_MAINNET!;
const RPC_URL_SEPOLIA = process.env.FORK_URL_SEPOLIA!;
const CACHE_PATH = "../tc-rs/tests/fixtures";

test("broadcast", async () => {
  console.log("Setup Railgun");
  const pool = JsPool.sepoliaEther1;
  const prover = createProver();
  const cache_syncer = JsSyncer.newCache(
    readFileSync(`${CACHE_PATH}/cache_sepolia_eth_1.json`, "utf-8")
  );
  const rpc_syncer = await JsSyncer.newRpc(RPC_URL_SEPOLIA, 10000n);
  const syncer = JsSyncer.newChained([cache_syncer, rpc_syncer]);

  const verifier = await JsVerifier.newRpc(RPC_URL_SEPOLIA);
  const relayerSyncer = await JsRelayerSyncer.newRpc(RPC_URL_MAINNET);
  const tornado = await JsBroadcastProvider.new(syncer, verifier, prover, relayerSyncer, RPC_URL_SEPOLIA);
  tornado.add_pool(pool);

  console.log("Setup viem");
  const publicClient = createPublicClient({
    chain: sepolia,
    transport: http(RPC_URL_SEPOLIA),
  });

  const account = privateKeyToAccount("0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80");
  const walletClient = createWalletClient({
    account,
    chain: sepolia,
    transport: http(RPC_URL_SEPOLIA),
  });

  await tornado.sync();

  // console.log("Testing Deposit");
  // const deposit = tornado.deposit(pool);
  // const note = deposit.note;
  // const depositTx = deposit.txData;

  // console.log("Sending deposit transaction");
  // const depositHash = await walletClient.sendTransaction({
  //   to: depositTx.to as `0x${string}`,
  //   data: depositTx.dataHex as `0x${string}`,
  //   value: BigInt(depositTx.value),
  // });
  // await publicClient.waitForTransactionReceipt({ hash: depositHash });

  // console.log("Syncing");
  // await tornado.sync();

  // console.log("Testing Withdraw");
  // const txhash = await tornado.broadcast(pool, note, account.address);
  // console.log("Broadcasted transaction with hash", txhash);
}, 300 * 1000);
