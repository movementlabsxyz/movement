# `scripts`
Movement scripts are organized in a particular pattern. 

## General Utility Scripts
These scripts are used for general utility purposes. They are not specific to any particular part of the Movement Network. They are organized under a topical directory name under the `scripts` directory.

## `movement` scripts
Are part of a standard movement flow. They are called from the `justfile` in the root of the repository. They are placed in the `movement` directory.

### `run`
This should be called with as `./scripts/movement/run <target> <'.' separated features> additional <process-compose:>`. This will run the `run` script for the target. 

The `run` script will:

1. First calls a `. ./scripts/prelude/$1`. It provides to this script the parsed '.' separated features.
2. Then it calls `./scripts/process-compose/$1` with the parsed features as overrides. Thus, the order of the features can change the behavior of the script.

This is the standard flow for running and testing components in the Movement Network.