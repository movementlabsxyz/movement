# ZKFP

Welcome to the Movement SDK ZKFP Project! This is an experimental implementation of Zero-Knowledge Fraud Proofs for the Move Language with the RISC Zero zkVM. For similar projects, check out out the following:
- [Layer-N ZKFP](https://www.layern.com/blog/zkfp)
- [AltLayer ZKFP](https://www.risczero.com/news/altlayer-zkfraudproofs)

## Quick Start

First, make sure [rustup] is installed. The
[`rust-toolchain.toml`][rust-toolchain] file will be used by `cargo` to
automatically install the correct version.

To build all methods and execute the method within the zkVM, run the following
command:

```bash
cargo run --bin host
```

### Organization
- RISC0 host code is located in the [`host`](./host) directory.
- RISC0 guest code, i.e. code that runs on the ZKVM, is located in the [`zkvm`](.guest) directory.
- A submodule for a modified version of the `move` is located in the [`vendors/move](./vendors/move) directory. This submodule is the `zkp` branch of our fork of the `move` repository.

### Toy Example

We are currently trying to get a toy example of running stateless Move execution working. A more complete document with notes on this phase of the project is available at [`docs/0-to-1.md`](./docs/0-to-1.md).


### Executing the project locally in development mode

During development, faster iteration upon code changes can be achieved by leveraging [dev-mode], we strongly suggest activating it during your early development phase. Furthermore, you might want to get insights into the execution statistics of your project, and this can be achieved by specifying the environment variable `RUST_LOG="[executor]=info"` before running your project.

Put together, the command to run your project in development mode while getting execution statistics is:

```bash
RUST_LOG="[executor]=info" RISC0_DEV_MODE=1 cargo run --bin host
```

### Running proofs remotely on Bonsai

_Note: The Bonsai proving service is still in early Alpha; an API key is
required for access. [Click here to request access][bonsai access]._

If you have access to the URL and API key to Bonsai you can run your proofs
remotely. To prove in Bonsai mode, invoke `cargo run` with two additional
environment variables:

```bash
BONSAI_API_KEY="YOUR_API_KEY" BONSAI_API_URL="BONSAI_URL" cargo run
```