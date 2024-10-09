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
- [`docker`](./docker): Dockerfiles for building Movement Labs software and Docker compose files for orchestrating services. See the [docker README](./docker/README.md) for more information about the organization of Dockerfiles.
- [`protocol-units`](./protocol-units): Protocol units for the Movement Network. These are the core building blocks of the Movement Network. See the [protocol-units README](./protocol-units/README.md) for more information about the organization of protocol units.
- [`networks`](./networks): Network runner entry points for the Movement Network. These are the entry points for running the Movement Network. See the [networks README](./networks/README.md) for more information about the organization of network runners.
- [`util`](./util): Utility crates for the Movement SDK. These crates provide useful functions, macros, and types for use in Movement SDK projects. See the [util README](./util/README.md) for more information about the organization of utility crates.
- [`proto`](./proto): Protocol buffer definitions for the Movement Network. These definitions are used to generate code for interacting with the Movement Network. See the [proto README](./proto/README.md) for more information about the organization of protocol buffer definitions.

## Naming Conventions
Because we are in early stages of the "movement" network we decided to version the
network using formula one track names ("Monaco", "Monza", "Suzuka") instead of a more 
classical semantic versioning. 

### Naming Conventions Latest: SUZUKA

## Prerequisites
### Prerequisites - Just command
`just` is a handy way to save and run project-specific commands. Please install it
[following just install instructions](https://github.com/casey/just?tab=readme-ov-file#installation). `macOS` and `debian` based systems instructions below.

### Just command - macOS
```bash 
brew install just
```
Check install
```bash
just --version
```

### Just command - debian
```bash 
sudo apt update && sudo apt install --yes just
```
Check install
```bash
just --version
```

### Prerequisites - Nix : the package manager

https://nix.dev/install-nix


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
    - `local`: Run a local Celesta Data Availability service. 
    - `test`: run the test suite for `suzuka-full-node`. (Can be combined with `local`. Exits on completion by default.)

```bash
# example test with local
just monza-full-node native build.setup.test.local
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

### `suzuka-full-node`

- **Features**:
    - `setup`: Run setup for new `suzuka-full-node` network with single node.
    - `local`: Run a local Celesta Data Availability service.

**Note:** Currently, both `setup` and `local` must be used. 
We only support running the `suzuka-full-node` with a local Celesta Data Availability 
service via Docker Compose.

```bash
# example setup with local
just suzuka-full-node docker-compose setup.local
```
Under the hood, `just` runs
```bash
# working directory = GIT_ROOT
GIT_ROOT=$(git rev-parse --show-toplevel)
docker compose --env-file .env \
               --file docker/compose/suzuka-full-node/docker-compose.yml \
               --file docker/compose/suzuka-full-node/docker-compose.setup-local.yml \
               --file docker/compose/suzuka-full-node/docker-compose.celestia-local.yml \
               up
```

**Note:** if you want to recreate the network, but not rely on the just target above, please read through the scripts to identify the correct `docker-compose` files to run.

**Note** For attesters in order to receive rewards you need to launch the node in 
`Attester` mode. To do this you will need to provide a private key at runtime.
This feature is not implemented yet, at this moment.


## Services

### `suzuka-full-node`

Both `native` and `docker-compose` runners will serve the following services listening on the specified default addresses:

**Note:** Only APIs intended for the end-user are listed here. For a full list of services, please refer to respective `docker-compose` files.

- **[Aptos REST API](https://api.devnet.aptoslabs.com/v1/spec#/)**: `0.0.0.0:30731`
- **[Aptos Faucet API](https://aptos.dev/apis/#faucet-api-only-testnetdevnet)**: `0.0.0.0:30732`

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](LICENSE) file for details.
