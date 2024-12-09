# Function to check and remove the .movement/ directory if it exists
remove-dot-movement:
    @if [ "${REMOVE_DOT_MOVEMENT}" = "true" ] && [ -d "${DOT_MOVEMENT_PATH}" ]; then \
        echo "Removing ${DOT_MOVEMENT_PATH} directory..."; \
        rm -rf "${DOT_MOVEMENT_PATH}"; \
    fi

# Commands with a dependency on `remove-dot-movement`
movement-celestia-da-light-node RUNTIME FEATURES *ARGS:
    just remove-dot-movement
    ./scripts/movement/run movement-celestia-da-light-node {{ RUNTIME }} {{ FEATURES }} {{ ARGS }}

monza-full-node RUNTIME FEATURES *ARGS:
    just remove-dot-movement
    ./scripts/movement/run monza-full-node {{ RUNTIME }} {{ FEATURES }} {{ ARGS }}

movement-full-node RUNTIME FEATURES *ARGS:
    just remove-dot-movement
    ./scripts/movement/run movement-full-node {{ RUNTIME }} {{ FEATURES }} {{ ARGS }}

mcr-contract-tests: 
    just remove-dot-movement
    cd ./protocol-units/settlement/mcr/contracts && forge test

mcr-client RUNTIME FEATURES *ARGS:
    just remove-dot-movement
    ./scripts/movement/run mcr-client {{ RUNTIME }} {{ FEATURES }} {{ ARGS }}

bridge RUNTIME FEATURES *ARGS:
    just remove-dot-movement
    ./scripts/movement/run bridge {{ RUNTIME }} {{ FEATURES }} {{ ARGS }}

bridge-solo RUNTIME FEATURES *ARGS:
    just remove-dot-movement
    ./scripts/movement/run bridge-solo {{ RUNTIME }} {{ FEATURES }} {{ ARGS }}

build-push-container IMAGE:
    just remove-dot-movement
    ./scripts/movement/build-push-image {{ IMAGE }}

container-tests:
    just remove-dot-movement
    ./scripts/tests/container-tests
