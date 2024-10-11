# Current Contract Deployments
## Holesky Ethereum Deployments

| Contract                         | Address                                      |
|-----------------------------------|----------------------------------------------|
| `AtomicBridgeInitiatorMove.sol`   | 0xb33d9fA868054FD3F8F02ba9Fc04B2a1862Ab855   |
| `AtomicBridgeCounterpartyMove.sol`    | 0xA6bB0d5eA03f3bEe3510EA3Ec39e85Cd7395Bc37   |


## testnet.movementlabs Deployments
the movement atomic-bridge and all of its modules are deployed under the same package:
`0xf7c46960ef7aa1b25cd5d48efef064525aa4ccec286fc3b3a7176bbfec09b84c`

Verify this by running:
```
 movement account list --account 0xf7c46960ef7aa1b25cd5d48efef064525aa4ccec286fc3b3a7176bbfec09b84c 
```
## Running the Atomic Bridge Relayer In Production
The atomic bridge relayer services requires a `config.json` at the root of the application inside a `./movement`

Set these values to your needs to configure the relayer.

```json
{
  "eth": {
    "eth_rpc_connection_protocol": "http",
    "eth_rpc_connection_hostname": "0.0.0.0",
    "eth_rpc_connection_port": 8090,
    "eth_ws_connection_protocol": "ws",
    "eth_ws_connection_hostname": "0.0.0.0",
    "eth_ws_connection_port": 8090,
    "eth_chain_id": 3073,
    "eth_initiator_contract": "0xb33d9fA868054FD3F8F02ba9Fc04B2a1862Ab855",
    "eth_counterparty_contract": "0xA6bB0d5eA03f3bEe3510EA3Ec39e85Cd7395Bc37",
    "eth_weth_contract": "0xB7f8BC63BbcaD18155201308C8f3540b07f84F5e",
    "signer_private_key": "<ETH_PRIVATE_KEY>",
    "time_lock_secs": 60,
    "gas_limit": 10000000000000000,
    "transaction_send_retries": 10
  },
  "movement": {
    "movement_signer_address": "<MOVEMENT_PRIVATE_KEY>",
    "movement_native_address": "<MOVEMENT_PUBLIC_KEY>",
    "mvt_rpc_connection_protocol": "http",
    "mvt_rpc_connection_hostname": "0.0.0.0",
    "mvt_rpc_connection_port": 30731,
    "mvt_faucet_connection_protocol": "http",
    "mvt_faucet_connection_hostname": "0.0.0.0",
    "mvt_faucet_connection_port": 30732,
    "mvt_init_network": "custom"
  },
  "testing": {
    "eth_well_known_account_private_keys": [
     <0th EL FROM ANVIL>,
     <1st EL FROM ANVIL>,
      ...
    ]
  }
}

```
Once the relayer is up and listening for events, you can initiate a transfer.

## Bridging MOVE from Ethereum to Movement without a UI

First we need to create the `preimage` or secret, for the transfer. 

1. `cast keccak256 <YOUR_SECRET>`

The secret can be any string. When doing this operation through the UI, a unique secret is automatically generate, if using a CLI, you should always use a unique secret for each transfer.

This will generate your `HASH_LOCK` value.

2. Now we call `initiateBridgeTransfer` on `AtomicBridgeInitiatorMOVE` with the cast CLI

```
cast send <CONTRACT_ADDRESS> "initiateBridgeTransfer(uint256,bytes32,bytes32)" <MOVE_AMOUNT> <RECIPIENT> <HASH_LOCK> --from <YOUR_ADDRESS> --rpc-url <RPC_URL>
```
Where 
- <CONTRACT_ADDRESS>: The deployed contract address.
<MOVE_AMOUNT>: The amount of MOVE tokens (in wei) to transfer.
- <RECIPIENT>: The recipient's address, formatted as a bytes32 string (you'll need to convert it to bytes32 beforehand).
- <HASH_LOCK>: The hash lock for the transfer, also a bytes32 value.
- <YOUR_ADDRESS>: Your wallet address, which will be the originator of the transaction.
- <RPC_URL>: The RPC URL of your Ethereum node or test network.

For example:
```
cast send 0xYourContractAddress "initiateBridgeTransfer(uint256,bytes32,bytes32)" 1000000000000000000 0x000000000000000000000000000000000000move 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef --from 0xYourWalletAddress --rpc-url http://127.0.0.1:8545
```
The relayer will now call functions on the movement atomic bridge contract. You can inspect the logs of the service to see its progress. 

Copy the `BridgeTransferId` in the logs. 
    
3. Now we call `complete_bridge_transfer` on the movement side. 
```
aptos move call \
  --function <MODULE_ADDRESS>::<MODULE_NAME>::complete_bridge_transfer \
  --type-args "" \
  --args <BRIDGE_TRANSFER_ID_HEX> <PRE_IMAGE_HEX>  \
  --profile <YOUR_PROFILE>
```

For example:
```
aptos move call \
  --function 0x1::BridgeModule::complete_bridge_transfer \
  --type-args "" \
  --args 0x1234 676d6f7665 \
  --profile default
```
You'll notice that the args in the module are of type `vec<u8>`, so you have to correctly convert any string into its hex representation. 

To demonstrate, let's say my `preimage` was `gmove`, if I run 
```
echo -n "gmove" | xxd -p
```
I get: `676d6f7665`

4. After `complete_bridge_transfer` has been completed on movement, you should now see your bridged assets in the account set as the `recipient` value in the `initiateBridgeTransfer` call, in this case it was `0x000000000000000000000000000000000000move`.
