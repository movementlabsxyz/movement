# `scripts`
Movement scripts are organized in a particular pattern. 

## General Utility Scripts
These scripts are used for general utility purposes. They are not specific to any particular part of the Movement Network. They are organized under a topical directory name under the `scripts` directory.

## `movement` scripts
Are part of a standard movement flow. They are called from the `justfile` in the root of the repository. They are placed in the `movement` directory.

### `run`
The main entry point for running services in this repository.

This should be called with as `./scripts/movement/run <service> <runtime> <'.' separated features> <*additional-flags-for-the-underlying-runtime>`. This will run the `run` script for the target. 

Run will then call the respective `./scripts/movement/<runtime>` script with all of the arguments passed to `run`.

### `docker-compose`
A `runtime` script that runs the service in a docker-compose environment. This script will call `docker-compose` with the appropriate arguments to run the service.

### `native` 
A `runtime` script that runs the service natively. This script will orchestrate the service with `process-compose` to run in the native environment.