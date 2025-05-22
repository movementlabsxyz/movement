# format-module-bytes

This tool prepares a Move package for publishing using the Movement Explorer UI or via Movement CLI with `movement multisig create-transaction`

It compiles the embedded Move package and outputs the required `arg0` and `arg1` values for the `code::publish_package_txn` function:

[https://explorer.movementlabs.xyz/account/0x0000000000000000000000000000000000000000000000000000000000000001/modules/run/code/publish\_package\_txn?network=mainnet](https://explorer.movementlabs.xyz/account/0x0000000000000000000000000000000000000000000000000000000000000001/modules/run/code/publish_package_txn?network=mainnet)

## Function Signature

```move
entry fun publish_package_txn(
  owner: &signer,
  metadata_serialized: vector<u8>,
  code: vector<vector<u8>>
)
```

## Usage

The easiest way to use the formatted module bytes is via Movement explorer.

1. Ensure the `movement` CLI is installed and available in your `PATH`. You also need to run `movement init` or directly add a `.movement/config.yaml` in the `util/format-module-bytes` dir.

2. Run from the workspace root:

   ```bash
   cargo run -p format-module-bytes
   ```

3. The tool will compile the embedded Move package and output two formatted arguments:

   * `arg0` (vector<u8>)
   * `arg1` (vector\<vector<u8>>)

   It will also write these to:

   ```
   format-module-bytes/build/hello/explorer_payload.log
   ```

## Submitting via the Explorer UI

1. Open the following link (for mainnet publishing... you can switch to testnet if you prefer):

   [https://explorer.movementlabs.xyz/account/0x0000000000000000000000000000000000000000000000000000000000000001/modules/run/code/publish\_package\_txn?network=mainnet](https://explorer.movementlabs.xyz/account/0x0000000000000000000000000000000000000000000000000000000000000001/modules/run/code/publish_package_txn?network=mainnet)

2. Input the args:

   * **signer**: your account address (must be funded)
   * **arg0**: the full vector<u8> array (surrounded by brackets)
   * **arg1**: the full vector\<vector<u8>> array (outer and inner brackets must be present)

   Example:

   ```json
   arg0: [5,104,101,108,108,111,...]
   arg1: [[161,28,235,11,...]]
   ```
   > [!TIP] Be sure to connect to the explorer with the same wallet that you used in your local Movement config.

   After successful publishing via explorer, you will see a successful message as follows:
  
   <img width="1205" alt="image" src="https://github.com/user-attachments/assets/312c2c17-e164-45d8-a7ca-c379ef0f21ed" />
