m1-da-light-node-test-native:
    cargo build -p m1-da-light-node
    scripts/movement/test-native m1-da-light-node
m1-da-light-node-run-native:
    cargo build -p m1-da-light-node
    scripts/movement/run-native m1-da-light-node