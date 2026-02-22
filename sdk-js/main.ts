import { readFile, writeFile } from "node:fs/promises";
import { createProverFunctions } from "./src/prover";
import { initWasm } from "./src/wasm";
import { createBroadcaster } from "./src/waku-transport";
import { createWalletClient, http } from "viem";
import { sepolia } from "viem/chains";
import { privateKeyToAccount } from "viem/accounts";
import { JsPoiProvider } from "./pkg/railgun_rs";

const hexKey = (fill: number): string => "0x" + fill.toString(16).padStart(2, "0").repeat(32);

const USDC_ADDRESS = "0x1c7d4b196cb0c7b01d743fbc6116a902379c7238";
const WETH_ADDRESS = "0xfff9976782d46cc05630d1f6ebab18b2324d6b14";
const CHAIN_ID = 11155111n;
const ARTIFACTS_PATH = "../railgun-rs/artifacts";
const PROVIDER_STATE_PATH = "../railgun-rs/provider_state_11155111.bincode";

const TEST_PRIVATE_KEY = process.env.DEV_KEY as string;
const RPC_URL = process.env.FORK_URL_SEPOLIA as string;
const SPENDING_KEY = process.env.DEV_SPENDING_KEY as string;
const VIEWING_KEY = process.env.DEV_VIEWING_KEY as string;

async function main() {
    console.log("Initializing WASM");
    const wasm = await initWasm();

    const broadcast_manager = await createBroadcaster(CHAIN_ID);
    broadcast_manager.start();

    const USDC = wasm.erc20_asset(USDC_ADDRESS);
    const WETH = wasm.erc20_asset(WETH_ADDRESS);

    console.log("Setting up prover");
    const { proveTransact, provePoi } = createProverFunctions({
        artifactsPath: ARTIFACTS_PATH,
    });
    const prover = new wasm.JsProver(proveTransact, provePoi);

    console.log("Setting up syncer");
    const subsquidSyncer = wasm.JsSyncer.withSubsquid("https://rail-squid.squids.live/squid-railgun-eth-sepolia-v2/v/v1/graphql");
    const rpcSyncer = await wasm.JsSyncer.withRpc(
        RPC_URL,
        CHAIN_ID,
        10n,
    );
    const syncer = wasm.JsSyncer.withChained([subsquidSyncer, rpcSyncer]);

    console.log("Setting up viem client");
    const walletClient = createWalletClient({
        account: privateKeyToAccount(`0x${TEST_PRIVATE_KEY}`),
        chain: sepolia,
        transport: http(RPC_URL),
    });
    console.log("Wallet address:", await walletClient.getAddresses());

    console.log("Setting up provider");
    const providerState = new Uint8Array(await readFile(PROVIDER_STATE_PATH));
    const railgun = await wasm.JsPoiProvider.from_state(
        providerState,
        RPC_URL,
        syncer,
        "https://rail-squid.squids.live/squid-railgun-eth-sepolia-v2/v/v1/graphql",
        prover,
    );
    railgun.reset_indexer();

    // const railgun = await wasm.JsPoiProvider.new(
    //     CHAIN_ID,
    //     RPC_URL,
    //     syncer,
    //     "https://rail-squid.squids.live/squid-railgun-eth-sepolia-v2/v/v1/graphql",
    //     prover,
    // )

    console.log("Setting up accounts");
    const account1 = new wasm.JsSigner(SPENDING_KEY, VIEWING_KEY, CHAIN_ID);
    const account2 = new wasm.JsSigner(hexKey(7), hexKey(8), CHAIN_ID);

    railgun.register(account1);
    railgun.register(account2);

    await railgun.sync();

    console.log("Saving provider state");
    await saveProviderState(railgun);

    const bal1 = railgun.balance(account1.address);
    console.log("Account 1 balance:");
    console.log("USDC: ", bal1.get(USDC));
    console.log("WETH: ", bal1.get(WETH));

    const bal2 = railgun.balance(account2.address);
    console.log("Account 2 balance:");
    console.log("USDC: ", bal2.get(USDC));
    console.log("WETH: ", bal2.get(WETH));

    // // let shield = railgun
    // //     .shield()
    // //     .shield(account1.address, USDC, "100")
    // //     .shield(account1.address, WETH, "100000000000000");
    // // let tx = shield.build();

    // // const shieldHash = await walletClient.sendTransaction({
    // //     to: tx.to as Address,
    // //     data: tx.dataHex as Hex,
    // //     value: BigInt(tx.value)
    // // });
    // // console.log("Shield tx hash:", shieldHash);

    // // console.log("Balance");
    // // let balance = railgun.balance(account1.address);
    // // console.log("USDC: ", balance.get(USDC));
    // // console.log("WETH: ", balance.get(WETH));

    console.log("Finding broadcaster");
    let broadcaster = undefined;
    while (!broadcaster) {
        await new Promise((resolve) => setTimeout(resolve, 1000));

        const unix_time = Math.floor(Date.now() / 1000);
        broadcaster = await broadcast_manager.best_broadcaster_for_token(WETH_ADDRESS, BigInt(unix_time));
        console.log("Waiting for broadcasters...");
    }

    console.log("Best broadcaster for WETH:", broadcaster);

    console.log("Creating transfer transaction");
    let builder = railgun.transact().transfer(
        account1,
        account2.address,
        USDC,
        "10",
        ""
    );

    console.log("Building transaction");
    let prepared = await railgun.build_broadcast(builder, account1, broadcaster.fee());

    console.log("Broadcasting transaction");
    // const txhash = await broadcaster.broadcast(prepared);
    // console.log("Broadcasted transaction with hash:", txhash);

    // await saveProviderState(railgun);
}

async function saveProviderState(railgun: JsPoiProvider) {
    const state = railgun.state();
    await writeFile(PROVIDER_STATE_PATH, state);
}

await main();
