import { test } from "vitest";
import { JsRailgunProvider, JsSyncer } from "../src/pkg/railgun_rs.js";
import { GrothProverAdapter } from "../src/prover-adapter.js";
import { writeFileSync } from "node:fs";
import { createPublicClient, http } from "viem";
import { mainnet } from "viem/chains";
import { ViemEthRpcAdapter } from "../../eth-rpc/src/viem.js";

const CHAIN_ID = 1n;
const RPC_URL = process.env.RPC_URL_MAINNET!;
const ARTIFACTS_PATH = "../../artifacts/railgun";
const FORK_BLOCK = 24379760n;

test("sync-utxo", async () => {
  const publicClient = createPublicClient({
    chain: mainnet,
    transport: http(RPC_URL),
  });

  console.log("Setup Railgun");
  const prover = new GrothProverAdapter({ artifactsPath: ARTIFACTS_PATH });
  const rpcAdapter = new ViemEthRpcAdapter(publicClient);
  const syncer = JsSyncer.newChained([
    JsSyncer.newSubsquid(CHAIN_ID),
    await JsSyncer.newRpc(rpcAdapter, CHAIN_ID, 10n),
  ])
  const railgun = await JsRailgunProvider.new(rpcAdapter, syncer, prover);

  console.log("Sync Railgun");
  await railgun.sync_to(FORK_BLOCK);

  // Save state to disk
  let state = railgun.state();
  writeFileSync("./provider_state_utxo_1.json", state);
}, 300 * 1000);
