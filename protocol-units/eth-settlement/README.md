# `m2`

## `eth-settlement`
To run the tests, first install the following dependencies:
- foundry

Start the Anvil local network:
```bash
anvil
```

Deploy the contracts in `contracts/eth-settlement`:
```bash
# from root of the project
cd contracts/eth-settlement
forge script script/DeploySettlement.s.sol --broadcast --rpc-url http://localhost:8545 --private-key <pick-key-from-anvil-avialable-accounts>
```

**Note**: Never use the private key above in production.

Set your environment variables:
```bash
ETH_RPC_URL=http://localhost:8545
ETH_CONTRACT_ADDRESS=<deployed-contract-address>
ETH_CONTRACT_ABI_PATH=../contracts/eth-settlement/out/Settlement.sol/Settlement.json
```

Run the tests:
```bash
# from root of the project
cd eth-settlement
cargo test test_eth_settlement_service_env
```