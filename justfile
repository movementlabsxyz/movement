movement-celestia-da-light-node RUNTIME FEATURES *ARGS:
    ./scripts/movement/run movement-celestia-da-light-node {{ RUNTIME }} {{ FEATURES }} {{ ARGS }}
monza-full-node RUNTIME FEATURES *ARGS:
    ./scripts/movement/run monza-full-node {{ RUNTIME }} {{ FEATURES }} {{ ARGS }}
movement-full-node RUNTIME FEATURES *ARGS:
    ./scripts/movement/run movement-full-node {{ RUNTIME }} {{ FEATURES }} {{ ARGS }}
mcr-contract-tests: 
    cd ./protocol-units/settlement/mcr/contracts && forge test
mcr-client RUNTIME FEATURES *ARGS:
    ./scripts/movement/run mcr-client {{ RUNTIME }} {{ FEATURES }} {{ ARGS }}
build-push-container IMAGE:
    ./scripts/movement/build-push-image {{ IMAGE }}
container-tests:
    ./scripts/tests/container-tests

# E2E Tests for GGP Deprecation
test-e2e-verify-collect-fee:
    process-compose -f process-compose/movement-full-node/process-compose.test-e2e-verify-collect-fee.yml up --wait --follow

test-e2e-framework-upgrade-collect-gas-fees:
    process-compose -f process-compose/movement-full-node/process-compose.test-e2e-framework-upgrade-collect-gas-fees.yml up --wait --follow
