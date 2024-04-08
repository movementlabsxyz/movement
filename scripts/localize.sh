#!/bin/bash
# Example script that could replace a placeholder in Cargo.toml.template
PWD=$(pwd)
sed "s|PWD|file://${PWD}/vendors/sibling-workspace|" Cargo.toml.template > Cargo.toml

# Then run your cargo build or cargo run commands
cargo build
