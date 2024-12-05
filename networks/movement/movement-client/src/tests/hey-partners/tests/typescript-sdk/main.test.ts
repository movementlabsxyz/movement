import { Aptos, AptosConfig, Account, Network, MoveModuleBytecode, AccountAddress, Ed25519PrivateKey, APTOS_COIN, HexInput } from "@aptos-labs/ts-sdk"
import dotenv from "dotenv"
// import 'mocha'
// import {expect} from "chai"

// Specify the path to the .env file in the parent directory
dotenv.config({ path: '.env' });

if (!process.env.FULLNODE || !process.env.FAUCET || !process.env.PRIVATE_KEY || !process.env.SWAP_DEPLOYER || !process.env.RESOURCE_ACCOUNT_DEPLOYER) process.exit(1);
const SwapDeployer = process.env.SWAP_DEPLOYER;
const ResourceAccount = process.env.RESOURCE_ACCOUNT_DEPLOYER;


const aptosConfig = new AptosConfig({ network: Network.DEVNET, fullnode: process.env.FULLNODE, faucet: process.env.FAUCET  });
const aptos = new Aptos(aptosConfig);

const privateKey = new Ed25519PrivateKey(process.env.PRIVATE_KEY as string);
const deployer = Account.fromPrivateKey({ privateKey });

const alice = Account.generate();
const bob = Account.generate();

const amount = 100000000;

async function transact(signer: any, func: `${string}::${string}::${string}`, typeArgs: string[], args: any[]) {
    const transaction = await aptos.transaction.build.simple({
        sender: signer.accountAddress,
        data: {
            function: func,
            typeArguments: [...typeArgs],
            functionArguments: [...args],
        },
    });
    const committedTransaction = await aptos.signAndSubmitTransaction({ signer: signer, transaction });
    return committedTransaction
}

async function main() {

    console.log("Requesting funds for Alice from Alice")
    try {
        // await aptos.faucet.fundAccount({ accountAddress: alice.accountAddress, amount: amount });
        await fetch(`${process.env.FAUCET}/mint`, { method: 'POST', body: JSON.stringify({ address: alice.accountAddress, amount: amount }) })
        console.log("Account funded successfully.");
      } catch (error : any) {
        console.log(error)
        console.error("Error funding account:", error);
        console.error("Error details:", error.message || error);
      }
    console.log("Requesting funds for Alice from Bob")
    // await aptos.faucet.fundAccount({ accountAddress: bob.accountAddress, amount: amount })
    await fetch(`${process.env.FAUCET}/mint`, { method: 'POST', body: JSON.stringify({ address: bob.accountAddress, amount: amount }) })

    // describe("sdk", async () => {
    //     test("transaction reads", async () => {

    console.log("Getting account info for Alice")
    const fund = await aptos.getAccountInfo({ accountAddress: alice.accountAddress });
    console.log("Getting account modules for deployer")
    const modules = await aptos.getAccountModules({ accountAddress: deployer.accountAddress });
    console.log("Getting account owned tokens for deployer")
    const tokens = await aptos.getAccountOwnedTokens({ accountAddress: deployer.accountAddress });
    console.log("Getting account resource for deployer")
    const testCoinsV1 = await aptos.getAccountResource<typeof APTOS_COIN>({
        accountAddress: deployer.accountAddress,
        resourceType: `0x1::coin::CoinStore<${deployer.accountAddress}::TestCoinsV1::USDT>`,
    })

    // expect(fund).to.be(String(amount));
    // expect(modules[0].abi?.name).to.be(`uq64x64`)
    // expect(tokens[0].token_standard).to.be('AptosCoin')
    // })


    // test("transaction writes", async () => {
    console.log("Minting USDT for deployer")
    const mint = await transact(deployer, `${SwapDeployer}::TestCoinV1::mint_coin`, [`${SwapDeployer}::TestCoinsV1::USDT`], [20000000000000000]);
    console.log("Requesting USDT for Alice")
    const usdt_request = await transact(alice, `${SwapDeployer}::FaucetV1::request`, [`${SwapDeployer}::TestCoinsV1::USDT`], [alice.accountAddress])
    console.log("Requesting USDT for Alice from Bob")
    const nice_request = await transact(bob, `${SwapDeployer}::FaucetV1::request`, [`${SwapDeployer}::TestCoinsV1::USDT`], [alice.accountAddress])

    console.log("Requesting BTC for Alice")
    const btc_request = await transact(alice, `${SwapDeployer}::FaucetV1::request`, [`${SwapDeployer}::TestCoinsV1::BTC`], [alice.accountAddress])
    // swap exact BTC
    console.log("Swapping BTC for exact MOVE")
    const btc_swap = await transact(alice, `${SwapDeployer}::AnimeSwapPoolV1::swap_exact_coins_for_coins_entry`, [`${SwapDeployer}::TestCoinsV1::BTC 0x1::aptos_coin::AptosCoin`], ['u64:100 u64:1000000000'])
    // swap BTC for exact USDT
    console.log("Swapping exact BTC for MOVE")
    const btc_swap_exact = await transact(alice, `${SwapDeployer}::AnimeSwapPoolV1::swap_exact_coins_for_coins_entry`, [`${SwapDeployer}::TestCoinsV1::BTC 0x1::aptos_coin::AptosCoin`, `0x1::aptos_coin::AptosCoin`, `${SwapDeployer}::TestCoinsV1::USDT`], ['u64:100 u64:1'])

    //         expect(mint.type).to.be(String(1))
    //         expect(usdt_request.type).to.be(String(1))
    //         expect(nice_request.type).to.be(String(1))
    //         expect(btc_request.type).to.be(String(1))
    //         expect(btc_swap.type).to.be(String(1))
    //         expect(btc_swap_exact.type).to.be(String(1))
    //     })
    // })

}
main().catch((err) => { console.log(err) })