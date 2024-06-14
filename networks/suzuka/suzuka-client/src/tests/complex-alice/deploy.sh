#!/bin/bash

# Initializes an account if keys are not present
echo "Initializing account"
initialize_output=$(echo -ne '\n' | aptos init --network custom --rest-url $NODE_URL --faucet-url $FAUCET_URL --assume-yes)
echo "$initialize_output"

echo "Running tests"
aptos move test --package-dir src/tests/complex-alice --named-addresses resource_roulette=default

echo "Compiling the module"
aptos move compile --package-dir src/tests/complex-alice --named-addresses resource_roulette=default

echo "Publishing the module"
aptos move publish --package-dir src/tests/complex-alice --named-addresses resource_roulette=default