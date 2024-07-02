m1-da-light-node RUNTIME FEATURES *ARGS:
    ./scripts/movement/run m1-da-light-node {{ RUNTIME }} {{ FEATURES }} {{ ARGS }}
monza-full-node RUNTIME FEATURES *ARGS:
    ./scripts/movement/run monza-full-node {{ RUNTIME }} {{ FEATURES }} {{ ARGS }}
suzuka-full-node RUNTIME FEATURES *ARGS:
    ./scripts/movement/run suzuka-full-node {{ RUNTIME }} {{ FEATURES }} {{ ARGS }}
mcr-contract-tests: 
    cd ./protocol-units/settlement/mcr/contracts && forge test
build-push-container IMAGE:
    ./scripts/movement/build-push-image {{ IMAGE }}
mcr RUNTIME FEATURES *ARGS:
    ./scripts/movement/run mcr {{ RUNTIME }} {{ FEATURES }} {{ ARGS }}
container-test:
    ./scripts/tests/container-test
