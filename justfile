m1-da-light-node RUNTIME FEATURES *ARGS:
    ./scripts/movement/run m1-da-light-node {{ RUNTIME }} {{ FEATURES }} {{ ARGS }}
monza-full-node RUNTIME FEATURES *ARGS:
    ./scripts/movement/run monza-full-node {{ RUNTIME }} {{ FEATURES }} {{ ARGS }}
suzuka-full-node RUNTIME FEATURES *ARGS:
    ./scripts/movement/run suzuka-full-node {{ RUNTIME }} {{ FEATURES }} {{ ARGS }}
mcr-contract-tests: 
    cd ./protocol-units/settlement/mcr/contracts && forge test
mcr-client RUNTIME FEATURES *ARGS:
    ./scripts/movement/run mcr-client {{ RUNTIME }} {{ FEATURES }} {{ ARGS }}
build-push-container IMAGE:
    ./scripts/movement/build-push-image {{ IMAGE }}
container-test:
    ./scripts/tests/container-test
