m1-da-light-node PATH *ARGS:
    cargo build -p m1-da-light-node
    scripts/movement/run m1-da-light-node {{ PATH }} {{ ARGS }}