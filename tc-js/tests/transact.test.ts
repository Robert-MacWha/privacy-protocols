import { createPublicClient, createWalletClient, http, parseAbi } from "viem";
import { expect, test } from "vitest";
import { createProver } from "../src/prover-adapter.js";
import { sepolia } from "viem/chains";
import { privateKeyToAccount } from "viem/accounts";
import { JsPool, JsSyncer, JsTornadoProvider } from "../src/pkg/tc_rs.js";
import { readFile } from "node:fs/promises";

const RPC_URL = "http://localhost:8545";
const CACHE_PATH = "../tc-rs/tests/fixtures";

test("transact-tc", async () => {
  console.log("Setup Railgun");
  const pool = JsPool.eth1;
  const prover = createProver();
  const cache_syncer = JsSyncer.newCache(
    await readFile(`${CACHE_PATH}/deposits_eth_1.json`, "utf-8"),
    await readFile(`${CACHE_PATH}/withdrawals_eth_1.json`, "utf-8")
  );
  const rpc_syncer = await JsSyncer.newRpc(RPC_URL, pool.address, 10000n);
  const syncer = JsSyncer.newChained([cache_syncer, rpc_syncer]);
  const tornado = await JsTornadoProvider.new(pool, RPC_URL, syncer, prover);

  console.log("Setup viem");
  const publicClient = createPublicClient({
    chain: sepolia,
    transport: http(RPC_URL),
  });

  const account = privateKeyToAccount("0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80");
  const walletClient = createWalletClient({
    account,
    chain: sepolia,
    transport: http(RPC_URL),
  });

  await tornado.sync();

  console.log("Testing Deposit");
  const deposit = tornado.deposit(pool);
  const note = deposit.note;
  const depositTx = deposit.txData;

  console.log("Sending deposit transaction");
  const depositHash = await walletClient.sendTransaction({
    to: depositTx.to as `0x${string}`,
    data: depositTx.dataHex as `0x${string}`,
    value: BigInt(depositTx.value),
  });
  await publicClient.waitForTransactionReceipt({ hash: depositHash });

  console.log("Syncing");
  await tornado.sync();

  console.log("Testing Withdraw");
  const toAddress = "0x1122334455667788990011223344556677889900";
  const withdrawTx = await tornado.withdraw(pool, note, account.address, toAddress, 0n, 0n);

  console.log("Sending withdraw transaction");
  const withdrawHash = await walletClient.sendTransaction({
    to: withdrawTx.to as `0x${string}`,
    data: withdrawTx.dataHex as `0x${string}`,
    value: BigInt(withdrawTx.value),
  });
  await publicClient.waitForTransactionReceipt({ hash: withdrawHash });
}, 300 * 1000);
