#!/bin/bash

# Initializes an account if keys are not present
echo "Initializing account"
initialize_output=$(echo -ne '\n' | movement init --network custom --rest-url $NODE_URL --faucet-url $FAUCET_URL --assume-yes)
echo "$initialize_output"

echo "Publishing the module"
movement move clean --assume-yes
movement move publish --package-dir src/tests/complex-alice --named-addresses resource_roulette=default --assume-yes