#!/bin/sh

# Initializes an account if keys are not present
initialize_output=$(echo -ne '\n' | aptos init --network custom --rest-url $NODE_URL --faucet-url $FAUCET_URL --assume-yes)
echo "$initialize_output"

aptos move test --package-dir src/tests/complex-alice --named-addresses resource_roulette=default

aptos move compile --package-dir src/tests/complex-alice --named-addresses resource_roulette=default