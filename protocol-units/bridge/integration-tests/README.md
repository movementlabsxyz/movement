# Running client integration tests against the native bridge

The client integration tests for the framework bridge modules are in `tests`:

- `client_l1move_l2move.rs` (run with command `rust_backtrace=1 cargo test --test client_l1move_l2move test_movement_client_initiate_transfer -- --nocapture --test-threads=1`)
- `client_l2move_l1move.rs` (run with command `rust_backtrace=1 cargo test --test client_l2move_l1move test_movement_client_initiate_transfer -- --nocapture --test-threads=1`)

In order to successfully run the tests against a local Suzuka node, the core resource account `0xA550C18` needs to be set to the same private key generated in `config.json`.

## Steps to run integration tests

1. Navigate to the root of the `movement` repo, checkout the `andygolay/framework-client-tests` branch, and start a local Suzuka node:

```
CELESTIA_LOG_LEVEL=FATAL CARGO_PROFILE=release CARGO_PROFILE_FLAGS=--release nix develop --extra-experimental-features nix-command --extra-experimental-features flakes --command bash  -c "just bridge native build.setup.eth-local.celestia-local --keep-tui"
```

Note, if you have any `.movement` directories present, e.g. when re-running tests, the directories must be deleted before starting the node. 

2. In the generated `.movement` directory, there will be a `config.yaml`. In that file, change `f90391c81027f03cdea491ed8b36ffaced26b6df208a9b569e5baf2590eb9b16` to `0xA550C18` so the file looks like:

```
---
profiles:
  default:
    network: Custom
    private_key: "0x0000000000000000000000000000000000000000000000000000000000000001"
    public_key: "0x4cb5abf6ad79fbf5abbccafcc269d85cd2651ed4b885b5869f241aedf0a5ba29"
    account: "0xA550C18"
    rest_url: "http://0.0.0.0:30731/"
    faucet_url: "http://0.0.0.0:30732/"
``` 

3. Run each set of Movement client tests:

L1 Move to L2 Move:
```
rust_backtrace=1 cargo test --test client_l1move_l2move -- --nocapture --test-threads=1
```

L2 Move to L1 Move:
```
rust_backtrace=1 cargo test --test client_l2move_l1move -- --nocapture --test-threads=1
```
