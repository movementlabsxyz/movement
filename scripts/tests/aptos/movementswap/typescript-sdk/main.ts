import { Aptos, AptosConfig, Account, AccountAddress, Ed25519PrivateKey, APTOS_COIN, HexInput } from "@aptos-labs/ts-sdk"
import dotenv from "dotenv"
dotenv.config()


const SwapDeployer = process.env.SWAP_DEPLOYER;
const ResourceAccount = process.env.RESOURCE_ACCOUNT_DEPLOYER;

const aptosConfig = new AptosConfig({ fullnode: process.env.FULLNODE || "https://aptos.devnet.m1.movementlabs.xyz" });
const aptos = new Aptos(aptosConfig);

const privateKey = new Ed25519PrivateKey(process.env.PRIVATE_KEY as HexInput);
const deployer = Account.fromPrivateKey({ privateKey });

const alice = Account.generate();
const bob = Account.generate();

const amount = 100000000;

aptos.faucet.fundAccount({accountAddress: alice.accountAddress, amount: amount})
aptos.faucet.fundAccount({accountAddress: bob.accountAddress, amount: amount})

// READ
const fund = await aptos.getAccountInfo({ accountAddress: alice.accountAddress });
const modules = await aptos.getAccountTransactions({ accountAddress: deployer.accountAddress });
const tokens = await aptos.getAccountOwnedTokens({ accountAddress: deployer.accountAddress });

const testCoinsV1 = await aptos.getAccountResource<typeof APTOS_COIN>({
    accountAddress: deployer.accountAddress,
    resourceType: `0x1::coin::CoinStore<${deployer.accountAddress}::TestCoinsV1::USDT>`,
})

async function transact(signer, func : `${string}::${string}::${string}`, typeArgs : string[], args) {
    const transaction = await aptos.transaction.build.simple({
        sender: signer.accountAddress,
        data: {
          function: func,
          typeArguments: [...typeArgs],
          functionArguments: [...args],
        },
      });
      const committedTransaction = await aptos.signAndSubmitTransaction({ signer: signer, transaction});
      return committedTransaction
}

// WRITE
const mint = await transact(deployer, `${SwapDeployer}::TestCoinV1::mint_coin`, [`${SwapDeployer}::TestCoinsV1::USDT`], [20000000000000000]);
// USDT
const usdt_request = await transact(alice, `${SwapDeployer}::FaucetV1::request`, [`${SwapDeployer}::TestCoinsV1::USDT`], [alice.accountAddress])
const nice_request = await transact(bob, `${SwapDeployer}::FaucetV1::request`, [`${SwapDeployer}::TestCoinsV1::USDT`], [alice.accountAddress])
const btc_request = await transact(alice, `${SwapDeployer}::FaucetV1::request`, [`${SwapDeployer}::TestCoinsV1::BTC`], [alice.accountAddress])

// swap exact BTC
const btc_swap = await transact(alice, `${SwapDeployer}::AnimeSwapPoolV1::swap_exact_coins_for_coins_entry`, [`${SwapDeployer}::TestCoinsV1::BTC 0x1::aptos_coin::AptosCoin`],['u64:100 u64:1000000000'])
// swap BTC for exact USDT
const btc_swap_exact = await transact(alice, `${SwapDeployer}::AnimeSwapPoolV1::swap_exact_coins_for_coins_entry`, [`${SwapDeployer}::TestCoinsV1::BTC 0x1::aptos_coin::AptosCoin`, `0x1::aptos_coin::AptosCoin`, `${SwapDeployer}::TestCoinsV1::USDT`],['u64:100 u64:1'])

