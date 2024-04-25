m1-da-light-node FEATURES *ARGS:
    cargo build -p m1-da-light-node
    scripts/movement/run m1-da-light-node {{ FEATURES }} {{ ARGS }}
monza-full-node FEATURES *ARGS:
    cargo build -p monza-full-node
    scripts/movement/run monza-full-node {{ FEATURES }} {{ ARGS }}