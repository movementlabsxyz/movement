m1-da-light-node FEATURES *ARGS:
    cargo build -p m1-da-light-node
    scripts/movement/run m1-da-light-node {{ FEATURES }} {{ ARGS }}
suzuka-full-node FEATURES *ARGS:
    scripts/movement/run suzuka-full-node {{ FEATURES }} {{ ARGS }}
mcr-contract-tests: 
    cd ./protocol-units/settlement/mcr/contracts && forge test