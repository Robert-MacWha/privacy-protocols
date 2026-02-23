import { readFile, writeFile } from "node:fs/promises";
import { createProver } from "./src/prover";
import { initWasm } from "./src/wasm";
import { createBroadcaster } from "./src/waku-transport";
import { createPublicClient, createWalletClient, Hex, http, parseAbi } from "viem";
import { sepolia } from "viem/chains";
import { Address, privateKeyToAccount } from "viem/accounts";
import { JsBroadcasterManager, JsPoiProvider, JsSigner } from "./pkg/railgun_rs";

const USDC_ADDRESS = "0x1c7d4b196cb0c7b01d743fbc6116a902379c7238";
const WETH_ADDRESS = "0xfff9976782d46cc05630d1f6ebab18b2324d6b14";
const CHAIN_ID = 11155111n;
const ARTIFACTS_PATH = "../railgun-rs/artifacts";
const PROVIDER_STATE_PATH = "./provider_state_11155111.json";

const TEST_PRIVATE_KEY = process.env.DEV_KEY as string;
const RPC_URL = process.env.FORK_URL_SEPOLIA as string;
const SUBSQUID_URL = "https://rail-squid.squids.live/squid-railgun-eth-sepolia-v2/v/v1/graphql";

const erc20Abi = parseAbi([
    "function balanceOf(address) view returns (uint256)",
]);

async function main() {
    console.log("Initializing WASM");
    const wasm = await initWasm();

    const USDC = wasm.erc20_asset(USDC_ADDRESS);
    const WETH = wasm.erc20_asset(WETH_ADDRESS);

    console.log("Setup");
    const publicClient = createPublicClient({
        chain: sepolia,
        transport: http(RPC_URL),
    });

    const walletClient = createWalletClient({
        account: privateKeyToAccount(`0x${TEST_PRIVATE_KEY}`),
        chain: sepolia,
        transport: http(RPC_URL),
    });
    const address = (await walletClient.getAddresses())[0];
    console.log("Wallet address:", address);

    if (!address) {
        throw new Error("No address found in wallet client");
    }

    const broadcast_manager = await createBroadcaster(CHAIN_ID);
    broadcast_manager.start();

    const prover = createProver({
        artifactsPath: ARTIFACTS_PATH,
    });

    const subsquidSyncer = wasm.JsSyncer.withSubsquid(SUBSQUID_URL);
    const rpcSyncer = await wasm.JsSyncer.withRpc(
        RPC_URL,
        CHAIN_ID,
        10n,
    );
    let railgun;
    try {
        railgun = await wasm.JsPoiProvider.from_state(
            await readFile(PROVIDER_STATE_PATH),
            RPC_URL,
            wasm.JsSyncer.withChained([subsquidSyncer, rpcSyncer]),
            SUBSQUID_URL,
            prover,
        )
    } catch (e) {
        railgun = await wasm.JsPoiProvider.new(
            CHAIN_ID,
            RPC_URL,
            wasm.JsSyncer.withChained([subsquidSyncer, rpcSyncer]),
            SUBSQUID_URL,
            prover,
        );
    }

    console.log("Setting up accounts");
    let account1 = wasm.JsSigner.random(CHAIN_ID);
    let account2 = wasm.JsSigner.random(CHAIN_ID);

    railgun.register(account1);
    railgun.register(account2);

    await railgun.sync();
    const state = railgun.state();
    await writeFile(PROVIDER_STATE_PATH, state);

    balance(railgun, account1, [USDC, WETH]);
    balance(railgun, account2, [USDC, WETH]);

    {
        console.log("Shielding assets to account 1");
        let shield = railgun
            .shield()
            .shield(account1.address, USDC, "100")
            .shield(account1.address, WETH, "10000000000000");
        let tx = shield.build();

        const shieldHash = await walletClient.sendTransaction({
            to: tx.to as Address,
            data: tx.dataHex as Hex,
            value: BigInt(tx.value)
        });
        await publicClient.waitForTransactionReceipt({ hash: shieldHash });
        console.log("Shield tx hash:", shieldHash);

    }

    // Wait for subsquid to index the txid (needed for POI submission)
    console.log("Waiting for POI to become valid...");
    await new Promise((resolve) => setTimeout(resolve, 80 * 1000));

    await railgun.sync();
    balance(railgun, account1, [USDC, WETH]);
    balance(railgun, account2, [USDC, WETH]);

    {
        console.log("Testing transfer");
        let broadcaster = await getBestBroadcaster(broadcast_manager);
        console.log("Using broadcaster:", broadcaster.address());
        let builder = railgun.transact().transfer(
            account1,
            account2.address,
            USDC,
            "10",
            ""
        );

        let prepared = await railgun.build_broadcast(builder, account1, broadcaster.fee());
        await railgun.broadcast(broadcaster, prepared);
        console.log("Transfer confirmed");

        await railgun.await_indexed(prepared);
        console.log("Transfer indexed");
    }

    await railgun.sync();
    balance(railgun, account1, [USDC, WETH]);
    balance(railgun, account2, [USDC, WETH]);

    {
        const preEoaBalance = await publicClient.readContract({
            address: USDC_ADDRESS as `0x${string}`,
            abi: erc20Abi,
            functionName: "balanceOf",
            args: [address],
        });
        console.log("Pre-unshield EOA USDC balance:", preEoaBalance);

        console.log("Testing unshield");
        let broadcaster = await getBestBroadcaster(broadcast_manager);
        console.log("Using broadcaster:", broadcaster.address());
        let builder = railgun.transact().unshield(
            account1,
            address,
            USDC,
            "5"
        );

        let prepared = await railgun.build_broadcast(builder, account1, broadcaster.fee());
        await railgun.broadcast(broadcaster, prepared);
        console.log("Unshield confirmed");

        await railgun.await_indexed(prepared);
        console.log("Unshield indexed");
    }

    await railgun.sync();
    balance(railgun, account1, [USDC, WETH]);
    balance(railgun, account2, [USDC, WETH]);

    const eoaBalance = await publicClient.readContract({
        address: USDC_ADDRESS as `0x${string}`,
        abi: erc20Abi,
        functionName: "balanceOf",
        args: [address],
    });
    console.log("Post-unshield EOA USDC balance:", eoaBalance);
}

function balance(railgun: JsPoiProvider, account: JsSigner, assets: string[]) {
    const bal = railgun.balance(account.address);
    console.log(`Account ${account.address} balance:`);
    for (const asset of assets) {
        console.log(`${asset}: `, bal.get(asset));
    }
}

async function getBestBroadcaster(broadcast_manager: JsBroadcasterManager) {
    let broadcaster = undefined;
    while (!broadcaster) {
        await new Promise((resolve) => setTimeout(resolve, 1000));

        const unix_time = Math.floor(Date.now() / 1000);
        broadcaster = await broadcast_manager.best_broadcaster_for_token(WETH_ADDRESS, BigInt(unix_time));
        console.log("Waiting for broadcasters...");
    }
    return broadcaster;
}

async function saveProviderState(railgun: JsPoiProvider) {
    const state = railgun.state();
    await writeFile(PROVIDER_STATE_PATH, state);
}

await main();
