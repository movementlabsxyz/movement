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

## Important
Unless otherwise specified assume all commands below are run after entering a nix shell with `nix develop`.  Development is made possible Within the Nix environment with all needed tooling being available.  If you are wishing to run the network then this can also be achieved without the need to install `nix` with having docker and [just](https://github.com/casey/just) installed only. 

## Organization
- [`scripts`](./scripts): Scripts for running Movement Labs software. See the [scripts README](./scripts/README.md) for more information about the organization of scripts.
- [`process-compose`](./process-compose): Process compose files for running Movement Labs software. These files are part of the standard flow for running and testing components in the Movement Network. See the [scripts README](./scripts/README.md) for more information about the organization of scripts.
- [`docker`](./docker): Dockerfiles for building Movement Labs software and Docker compose files for orchestrating services. See the [docker README](./docker/README.md) for more information about the organization of Dockerfiles.
- [`protocol-units`](./protocol-units): Protocol units for the Movement Network. These are the core building blocks of the Movement Network. See the [protocol-units README](./protocol-units/README.md) for more information about the organization of protocol units.
- [`networks`](./networks): Network runner entry points for the Movement Network. These are the entry points for running the Movement Network. See the [networks README](./networks/README.md) for more information about the organization of network runners.
- [`util`](./util): Utility crates for the Movement SDK. These crates provide useful functions, macros, and types for use in Movement SDK projects. See the [util README](./util/README.md) for more information about the organization of utility crates.
- [`proto`](./proto): Protocol buffer definitions for the Movement Network. These definitions are used to generate code for interacting with the Movement Network. See the [proto README](./proto/README.md) for more information about the organization of protocol buffer definitions.

## Prerequisites (Development)
- Nix package manager. Use nix to run and build Movement developer environments.  https://nix.dev/install-nix or https://determinate.systems/posts/determinate-nix-installer/.

## Prerequisites (Running Node)
- Docker and Docker Compose
- just https://github.com/casey/just

## Running Natively (Nix required)

### `movement-celestia-da-light-node`

- **Features**:
    - `build`: Build the `movement-celestia-da-light-node` binaries.
    - `setup`: Run setup for new `movement-celestia-da-light-node` network with single node.
    - `local`: Run a local Celestia Data Availability service. (Default.)
    - `arabica`: Run an Arabica Celestia Data Availability service. (Overrides local.)
    - `test`: Run the test suite for the `movement-celestia-da-light-node`. (Can be combined with `local` or `arabica`. Exits on completion by default.)

```bash
# example test with local  Celestia Data Availability service
just movement-celestia-da-light-node native build.setup.test.local
```

### `movement-full-node`

- **Features**:
    - `build`: Build the `movement-full-node` binaries.
    - `celestia-arabica`: DA on Celestia's Arabica network
    - `celestia-local`: Run a local Celesta Data Availability service.
    - `celestia-mocha`: DA on Celestia's Mocha network
    - `eth-local`: Settlement on a local Ethereum network
    - `eth-holesky`: Settlement on a Holesky Ethereum network
    - `setup`: Run setup for new `movement-full-node` network with single node.
    - `indexer`: Run a local indexer
    - `test`: run the test suite for `movement-full-node`. (Can be combined with `local`. Exits on completion by default.)

```bash
# example test with local celestia and local ethereum
just movement-full-node native build.setup.celestia-local.eth-local
```

## Run a Movement Node with Docker Compose
1. Make sure you have installed the `just` command on your system. If not check the 
"Prerequisites" section of this repo.

2. When running with `docker compose` specify your revision in a file `.env` at the root of
the project. The file should look like this:
```bash
# /path/to/movement/.env
CONTAINER_REV=0fe2a4f28820c04ca0db07cdd44cafc98b792f3f
```

We recommend to use the latest commit of the "main" branch:
```bash
GIT_ROOT=$(git rev-parse --show-toplevel)
MOVEMENT_ENV_FILE="${GIT_ROOT}/.env"
[[ -n "${GIT_ROOT}" ]] \
  && echo  "CONTAINER_REV=$(git rev-parse HEAD)" > "${MOVEMENT_ENV_FILE}"
echo "INFO: movement version is $(cat ${MOVEMENT_ENV_FILE})"
```

### `movement-full-node`

- **Features**:
    - `local`: Run a local Celesta Data Availability service.

We only support running the `movement-full-node` with a local Celestia Data Availability 
service via Docker Compose.

```bash
# A local Movement network
just movement-full-node docker-compose local
```
Under the hood, `just` runs
```bash
# working directory = GIT_ROOT
GIT_ROOT=$(git rev-parse --show-toplevel)
docker compose --env-file .env \
               --file docker/compose/movement-full-node/docker-compose.yml \
               --file docker/compose/movement-full-node/docker-compose.local.yml \
               up
```

**Note:** if you want to recreate the network, but not rely on the just target above, please read through the scripts to identify the correct `docker-compose` files to run.

**Note:** If you are experiencing any issues starting the local network please remove the local `./.movement` folder

**Note** For attesters in order to receive rewards you need to launch the node in 
`Attester` mode. To do this you will need to provide a private key at runtime.
This feature is not implemented yet, at this moment.

## Services

### `movement-full-node`

Both `native` and `docker-compose` runners will serve the following services listening on the specified default addresses:

**Note:** Only APIs intended for the end-user are listed here. For a full list of services, please refer to respective `docker-compose` files.

- **[Aptos REST API](https://api.devnet.aptoslabs.com/v1/spec#/)**: `0.0.0.0:30731`
- **[Aptos Faucet API](https://aptos.dev/apis/#faucet-api-only-testnetdevnet)**: `0.0.0.0:30732`

## Node Operation
For node operation guides, please begin with the manual [node operation docs](./docs/movement-node/run/manual/README.md).

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](LICENSE) file for details.
