# `scripts`
Movement scripts are organized in a particular pattern. 

## General Utility Scripts
These scripts are used for general utility purposes. They are not specific to any particular part of the Movement Network. They are placed in the root of the `scripts` directory.

## `movement` scripts
Are part of a standard movement flow. They are called from the `justfile` in the root of the repository. They are placed in the `movement` directory.

### `run-native` and `test-native`
1. These scripts dispatch first a `./scripts/prelude` with the argument `run` or `test`.
2. They then call run a `process-compose` script that will be located at `process-compose/<target>/<run|test>/process-compose.yml`.

This is the standard flow for running and testing components in the Movement Network.