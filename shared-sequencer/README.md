
[<img alt="crates.io" src="https://img.shields.io/crates/v/timestampvm.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/timestampvm)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-timestampvm-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/timestampvm)
![Github Actions](https://github.com/ava-labs/timestampvm-rs/actions/workflows/test-and-release.yml/badge.svg)

# `timestampvm-rs`

`timestampvm-rs` is a virtual machine that can build blocks from a user-provided arbitrary data. It is a minimal implementation of an Avalanche custom virtual machine (VM) in Rust, using the Avalanche [Rust SDK](https://github.com/ava-labs/avalanche-types-rs).

Currently, Avalanche custom VM requires the following:

1. Compiled to a binary that `avalanchego` can launch as a sub-process.
2. Plugin binary path in hash of 32 bytes.
3. Implements [`snowman.block.ChainVM`](https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/engine/snowman/block#ChainVM) interface that can be be registered via [`rpcchainvm.Serve`](https://pkg.go.dev/github.com/ava-labs/avalanchego/vms/rpcchainvm#Serve).
4. Implements VM-specific services that can be served via URL path of the blockchain ID.
5. (Optionally) Implements VM-specific static handlers that can be served via URL path of the VM ID.

For example, the timestamp VM can be run as follows:

```rust
use avalanche_types::subnet;
use timestampvm::vm;
use tokio::sync::broadcast::{self, Receiver, Sender};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let (stop_ch_tx, stop_ch_rx): (Sender<()>, Receiver<()>) = broadcast::channel(1);
    let vm_server = subnet::rpc::vm::server::Server::new(vm::Vm::new(), stop_ch_tx);
    subnet::rpc::plugin::serve(vm_server, stop_ch_rx).await
}
```

See [`bin/timestampvm`](timestampvm/src/bin/timestampvm/main.rs) for plugin implementation and [`tests/e2e`](tests/e2e/src/tests/mod.rs) for full end-to-end tests.

## Dependencies

- Latest version of stable Rust.
- To build and test timestampvm you need [protoc](https://grpc.io/docs/protoc-installation/#install-pre-compiled-binaries-any-os) version >= 3.15.0.

## AvalancheGo Compatibility
| Version(s) | AvalancheGo Version(s) |
| --- | --- |
| v0.0.6  | v1.9.2,v1.9.3 |
| v0.0.7  | v1.9.4 |
| v0.0.8, v0.0.9  | v1.9.7 |
| v0.0.10 | v1.9.8, v1.9.9 |
| v0.0.11,12 | v1.9.10 - v1.9.16 |
| v0.0.13 | v1.10.0 |
| v0.0.14..17 | v1.10.1 |
| v0.0.18 | v1.10.9+ |

## Example

```bash
# build the timestampvm plugin, run e2e tests, and keep the network running
./scripts/build.release.sh \
&& VM_PLUGIN_PATH=$(pwd)/target/release/timestampvm \
./scripts/tests.e2e.sh

# or, specify the custom avalanchego binary
./scripts/build.release.sh \
&& VM_PLUGIN_PATH=$(pwd)/target/release/timestampvm \
./scripts/tests.e2e.sh ~/go/src/github.com/ava-labs/avalanchego/build/avalanchego

# (optional) set NETWORK_RUNNER_ENABLE_SHUTDOWN=1 in "tests.e2e.sh"
# to shut down the network afterwards
```

To test `timestampvm` APIs, try the following commands:

```bash
# "tGas3T58KzdjcJ2iKSyiYsWiqYctRXaPTqBCA11BqEkNg8kPc" is the Vm Id
# e.g., timestampvm vm-id timestampvm
curl -X POST --data '{
    "jsonrpc": "2.0",
    "id"     : 1,
    "method" : "timestampvm.ping",
    "params" : []
}' -H 'content-type:application/json;' 127.0.0.1:9650/ext/vm/tGas3T58KzdjcJ2iKSyiYsWiqYctRXaPTqBCA11BqEkNg8kPc/static

# {"jsonrpc":"2.0","result":{"success":true},"id":1}
```

```bash
# "2wb1UXxAstB8ywwv4rU2rFCjLgXnhT44hbLPbwpQoGvFb2wRR7" is the blockchain Id
curl -X POST --data '{
    "jsonrpc": "2.0",
    "id"     : 1,
    "method" : "timestampvm.ping",
    "params" : []
}' -H 'content-type:application/json;' 127.0.0.1:9650/ext/bc/2wb1UXxAstB8ywwv4rU2rFCjLgXnhT44hbLPbwpQoGvFb2wRR7/rpc

# {"jsonrpc":"2.0","result":{"success":true},"id":1}
```

```bash
# to get genesis block
# "2wb1UXxAstB8ywwv4rU2rFCjLgXnhT44hbLPbwpQoGvFb2wRR7" is the blockchain Id
curl -X POST --data '{
    "jsonrpc": "2.0",
    "id"     : 1,
    "method" : "timestampvm.lastAccepted",
    "params" : []
}' -H 'content-type:application/json;' 127.0.0.1:9650/ext/bc/2wb1UXxAstB8ywwv4rU2rFCjLgXnhT44hbLPbwpQoGvFb2wRR7/rpc

# {"jsonrpc":"2.0","result":{"id":"SDfFUzkdzWZbJ6YMysPPNEF5dWLp9q35mEMaLa8Ha2w9aMKoC"},"id":1}

# "2wb1UXxAstB8ywwv4rU2rFCjLgXnhT44hbLPbwpQoGvFb2wRR7" is the blockchain Id
curl -X POST --data '{
    "jsonrpc": "2.0",
    "id"     : 1,
    "method" : "timestampvm.getBlock",
    "params" : [{"id":"SDfFUzkdzWZbJ6YMysPPNEF5dWLp9q35mEMaLa8Ha2w9aMKoC"}]
}' -H 'content-type:application/json;' 127.0.0.1:9650/ext/bc/2wb1UXxAstB8ywwv4rU2rFCjLgXnhT44hbLPbwpQoGvFb2wRR7/rpc

# {"jsonrpc":"2.0","result":{"block":{"data":"0x32596655705939524358","height":0,"parent_id":"11111111111111111111111111111111LpoYY","timestamp":0}},"id":1}
```

```bash
# to propose data
echo 1 | base64 | tr -d \\n
# MQo=

curl -X POST --data '{
    "jsonrpc": "2.0",
    "id"     : 1,
    "method" : "timestampvm.proposeBlock",
    "params" : [{"data":"MQo="}]
}' -H 'content-type:application/json;' 127.0.0.1:9650/ext/bc/2wb1UXxAstB8ywwv4rU2rFCjLgXnhT44hbLPbwpQoGvFb2wRR7/rpc

# {"jsonrpc":"2.0","result":{"success":true},"id":1}
```
