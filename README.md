<a href="https://movementlabs.xyz/">
  <h1 align="center">
      <img alt="Movement" src="./img/movement-labs-logo-yellow.png">
  </h1>
</a>

[![Discord badge][]](https://discord.gg/movementlabsxyz)
[![Twitter handle][]][Twitter badge]

[Discord badge]: https://img.shields.io/discord/1101576619493167217?logo=discord
[Twitter handle]: https://img.shields.io/twitter/follow/movementlabsxyz.svg?style=social&label=Follow
[Twitter badge]: https://twitter.com/intent/follow?screen_name=movementlabsxyz

The Movement SDK is a collection of tools and libraries for building, deploying, and working with Movement Labs infrastructure. The SDK is designed to be modular and extensible, allowing developers to build custom tools and libraries on top of the core components as well as to interact with Movement Labs' own networks.

**Note:** unless otherwise specified assume all commands below are run after entering a nix shell with `nix develop`.

## Organization
- [`scripts`](./scripts): Scripts for running Movement Labs software. See the [scripts README](./scripts/README.md) for more information about the organization of scripts.
- [`process-compose`](./process-compose): Process compose files for running Movement Labs software. These files are part of the standard flow for running and testing components in the Movement Network. See the [scripts README](./scripts/README.md) for more information about the organization of scripts.
- [`protocol-units`](./protocol-units): Protocol units for the Movement Network. These are the core building blocks of the Movement Network. See the [protocol-units README](./protocol-units/README.md) for more information about the organization of protocol units.
- [`util`](./util): Utility crates for the Movement SDK. These crates provide useful functions, macros, and types for use in Movement SDK projects. See the [util README](./util/README.md) for more information about the organization of utility crates.
- [`proto`](./proto): Protocol buffer definitions for the Movement Network. These definitions are used to generate code for interacting with the Movement Network. See the [proto README](./proto/README.md) for more information about the organization of protocol buffer definitions.

## Running Natively
### `m1-da-light-node`

- **Features**:
    - `build`: Build the `m1-da-light-node` binaries.
    - `setup`: Run setup for new `m1-da-light-node` network with single node.
    - `local`: Run a local Celestia Data Availability service. (Default.)
    - `arabica`: Run an Arabica Celestia Data Availability service. (Overrides local.)
    - `test`: Run the test suite for the `m1-da-light-node`. (Can be combined with `local` or `arabica`. Exits on completion by default.)

```bash
# example test with local  Celestia Data Availability service
just m1-da-light-node native build.setup.test.local
```

### `suzuka-full-node`

- **Features**:
    - `build`: Build the `suzuka-full-node` binaries.
    - `setup`: Run setup for new `suzuka-full-node` network with single node.
    - `local`: Run a local Celestia Data Availability service. 
    - `test`: run the test suite for `suzuka-full-node`. (Can be combined with `local`. Exits on completion by default.)

```bash
# example test with local
just monza-full-node native build.setup.test.local
```

## Run with Docker Compose
When running with `docker compose` specif your revision in a file `.env` at the root of the project. The file should look like this:

```bash
CONTAINER_REV=0fe2a4f28820c04ca0db07cdd44cafc98b792f3f
```

### `suzuka-full-node`

- **Features**:
    - `setup`: Run setup for new `suzuka-full-node` network with single node.
    - `local`: Run a local Celestia Data Availability service.

**Note:** Currently, both `setup` and `local` must be used. We only support running the `suzuka-full-node` with a local Celestia Data Availability service via Docker Compose.

```bash
# example setup with local
just suzuka-full-node docker-compose setup.local
```

**Note:** if you want to recreate the network, but not rely on the just target above, please read through the scripts to identify the correct `docker-compose` files to run.

## Services

### `suzuka-full-node`

Both `native` and `docker-compose` runners will serve the following services listening on the specified default addresses:

**Note:** Only APIs intended for the end-user are listed here. For a full list of services, please refer to respective `docker-compose` files.

- **[Aptos REST API](https://api.devnet.aptoslabs.com/v1/spec#/)**: `0.0.0.0:30731`
- **[Aptos Faucet API](https://aptos.dev/apis/#faucet-api-only-testnetdevnet)**: `0.0.0.0:30732`

## Troubleshooting

### `cp: cannot stat '': No such file or directory` when running `just suzuka-full-node native build.setup.test.local`

Try the following `nix.conf`:

```bash
build-users-group = nixbld
experimental-features = nix-command flakes repl-flake
bash-prompt-prefix = (nix:$name)\040
max-jobs = auto
extra-nix-path = nixpkgs=flake:nixpkgs
upgrade-nix-store-path-url = https://install.determinate.systems/nix-upgrade/stable/universal
```

### `ledger_info.is_some()`
The current revision does not support graceful restarts of the network from an existing config. For testing purposes, please delete you `.movement` directory and run the setup again.

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](LICENSE) file for details.
