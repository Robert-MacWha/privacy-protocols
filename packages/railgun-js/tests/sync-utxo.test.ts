import { test } from "vitest";
import { JsRailgunProvider } from "../src/pkg/railgun_rs.js";
import { createProver } from "../src/prover-adapter.js";
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
  const prover = createProver({ artifactsPath: ARTIFACTS_PATH });
  const rpcAdapter = new ViemEthRpcAdapter(publicClient);
  const railgun = await JsRailgunProvider.new_from_rpc(CHAIN_ID, rpcAdapter, 10n, prover);

  console.log("Sync Railgun");
  await railgun.sync_to(FORK_BLOCK);

  // Save state to disk
  let state = railgun.state();
  writeFileSync("./provider_state_utxo_1.json", state);
}, 300 * 1000);
