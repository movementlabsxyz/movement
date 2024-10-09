# Running client integration tests against the atomic bridge

The client integration tests for the framework bridge modules are in `tests`:

- `client_l1move_l2move.rs` (run with command `rust_backtrace=1 cargo test --test client_l1move_l2move test_movement_client_initiate_transfer -- --nocapture --test-threads=1`)
- `client_l2move_l1move.rs` (run with command `rust_backtrace=1 cargo test --test client_l2move_l1move test_movement_client_initiate_transfer -- --nocapture --test-threads=1`)

In order to successfully run the tests against a local Suzuka node, the core resource account `0xA550C18` needs to be set to the same private key generated in `config.json`.

## Steps to configure core resource account for signing in tests:

1. From the root of the `movement` repository, run the local Suzuka node:

```
CELESTIA_LOG_LEVEL=FATAL CARGO_PROFILE=release CARGO_PROFILE_FLAGS=--release nix develop --extra-experimental-features nix-command --extra-experimental-features flakes --command bash  -c "just bridge native build.setup.eth-local.celestia-local --keep-tui"
```

(You'll need Nix and Movement CLI if you don't already have them installed.)

This will generate a `.movement` dir in the project root containing a `config.json` with a `maptos_private_key` of `0x1`:
```
      "maptos_private_key": "0x0000000000000000000000000000000000000000000000000000000000000001",
```

There will also be a `config.yaml` that looks something like:

```
---
profiles:
  default:
    network: Custom
    private_key: "0x5754431205b8abc443a7a877a70d6e5e67eba8e5e40b0436bff5a9b6ab4a7887"
    public_key: "0x2b8c073bf4c091649d8fb5c275cacc6c8cf8cb6baaf0d7dffc47216011b6a27d"
    account: e813b12fc00bed33b54b5652c3bb1cbf12a33080aba9cd12d919b6d65cec6115
    rest_url: "http://localhost:30731/v1"
    faucet_url: "http://localhost:30732/"
```

2. Init a profile with the `0x1` private key. Here we init a profile named `local_root`:

```
movement init --profile local_root --rest-url http://localhost:30731/v1 --private-key 0x0000000000000000000000000000000000000000000000000000000000000001 --encoding hex --network custom
```

3. In `config.yaml` replace the `account` of `local_root` with `000000000000000000000000000000000000000000000000000000000a550c18` and skip the faucet step by pressing Enter when prompted. Your `config.yaml` will look like this:

```
---
profiles:
  default:
    network: Custom
    private_key: "<some-private-key>"
    public_key: "0x2b8c073bf4c091649d8fb5c275cacc6c8cf8cb6baaf0d7dffc47216011b6a27d"
    account: e813b12fc00bed33b54b5652c3bb1cbf12a33080aba9cd12d919b6d65cec6115
    rest_url: "http://localhost:30731/v1"
    faucet_url: "http://localhost:30732/"
  local_root:
    network: Custom
    private_key: "0x0000000000000000000000000000000000000000000000000000000000000001"
    public_key: "0x4cb5abf6ad79fbf5abbccafcc269d85cd2651ed4b885b5869f241aedf0a5ba29"
    account: 000000000000000000000000000000000000000000000000000000000a550c18
    rest_url: "http://localhost:30731/v1"
```

4. List the contents of the `0xa550c18` account:

```
 movement account list --account 0xA550C18
```

You should see a large number of `AptosCoin` held by the account:

```
{
      "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>": {
        "coin": {
          "value": "18446743973709450015"
        },
        "deposit_events": {
          "counter": "1",
          "guid": {
            "id": {
              "addr": "0xa550c18",
              "creation_num": "2"
            }
          }
        },
        "frozen": false,
        "withdraw_events": {
          "counter": "1",
          "guid": {
            "id": {
              "addr": "0xa550c18",
              "creation_num": "3"
            }
          }
        }
      }
    }
```

This large number of gas coins signifies that this is the core resource account.

5. Verify that you control the core resources signer by transferring gas tokens to your default account:

```
movement account transfer --profile local_root --amount 100 --account e813b12fc00bed33b54b5652c3bb1cbf12a33080aba9cd12d919b6d65cec6115
```

You'll be prompted to submit a transaction. After doing so, if the transaction is successful, you'll get a success message and transaction result.

## Steps to run client integration tests

Now that the core resource signer is configured, you can run the client integration tests.

There are two scripts in `protocol-units/bridge/move-modules`, one for enabling the Atomic Bridge feature flag and one for updating bridge operator.

1. Compile the scripts:

```
movement move compile --package-dir protocol-units/bridge/move-modules
```

2. Run the `enable_bridge_feature` script:

```
 movement move run-script --compiled-script-path protocol-units/bridge/move-modules/build/bridge-modules/bytecode_scripts/enable_bridge_feature.mv --profile local_root
 ```