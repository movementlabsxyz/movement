# `movement-sdk`
The Movement SDK is a collection of tools and libraries for building, deploying, and working with Movement Labs infrastructure. The SDK is designed to be modular and extensible, allowing developers to build custom tools and libraries on top of the core components as well as to interact with Movement Labs' own networks.

**Note:** unless otherwise specified assume all commands below are run after entering a nix shell with `nix develop`.

## Organization
- [`scripts`](./scripts): Scripts for running Movement Labs software. See the [scripts README](./scripts/README.md) for more information about the organization of scripts.
- [`process-compose`](./process-compose): Process compose files for running Movement Labs software. These files are part of the standard flow for running and testing components in the Movement Network. See the [scripts README](./scripts/README.md) for more information about the organization of scripts.
- [`protocol-units`](./protocol-units): Protocol units for the Movement Network. These are the core building blocks of the Movement Network. See the [protocol-units README](./protocol-units/README.md) for more information about the organization of protocol units.
- [`util`](./util): Utility crates for the Movement SDK. These crates provide useful functions, macros, and types for use in Movement SDK projects. See the [util README](./util/README.md) for more information about the organization of utility crates.
- [`proto`](./proto): Protocol buffer definitions for the Movement Network. These definitions are used to generate code for interacting with the Movement Network. See the [proto README](./proto/README.md) for more information about the organization of protocol buffer definitions.

# `m1-da-light-node`

- **Features**:
    - `local`: Run a local Celestia Data Availability service. (Default.)
    - `arabica`: Run an Arabica Celestia Data Availability service. (Overrides local.)
    - `test`: Run the test suite for the `m1-da-light-node`. (Can be combined with `local` or `arabica`. Exits on completion by default.)

```bash
# example test with local  Celestia Data Availability service
just m1-da-light-node test.local
```

# `monza-full-node`

- **Features**:
    - `local`: Run a local Celesta Data Availability service. 
    - `test`: run the test suite for `monza-full-node`. (Can be combined with `local`. Exits on completion by default.)

```bash
# example test with local
just monza-full-node test.local
```

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](LICENSE) file for details.
