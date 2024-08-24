#!/bin/bash

MOVE_TOML_PATH="protocol-units/bridge/move-modules/Move.toml"

# Initialize Aptos and capture output
INIT_OUTPUT=$(aptos init 2>&1)
echo "$INIT_OUTPUT"

# Extract the account address from the initialization output
ADDRESS=$(echo "$INIT_OUTPUT" | grep -oE '0x[a-f0-9]{64}' | head -1)
if [[ -z "$ADDRESS" ]]; then
    echo "Error: Failed to extract the Aptos account address."
    exit 1
fi

# Generate a random seed
RANDOM_SEED=$(shuf -i 0-1000000 -n 1)

# Derive the resource account address using the random seed
RESOURCE_OUTPUT=$(aptos account derive-resource-account-address --address "$ADDRESS" --seed "$RANDOM_SEED" 2>&1)
echo "Resource address derivation output: $RESOURCE_OUTPUT"

# Extract the resource address directly
RESOURCE_ADDRESS=$(echo "$RESOURCE_OUTPUT" | grep -oE '[a-f0-9]{64}')

if [[ -z "$RESOURCE_ADDRESS" ]]; then
    echo "Error: Failed to extract the resource account address."
    exit 1
fi

# Prepend the 0x to the resource address
RESOURCE_ADDRESS="0x$RESOURCE_ADDRESS"

echo "Extracted address: $ADDRESS"
echo "Derived resource address: $RESOURCE_ADDRESS"

# Update the Move.toml file with the addresses
sed -i "s/^resource_addr = \".*\"/resource_addr = \"$RESOURCE_ADDRESS\"/" "$MOVE_TOML_PATH"
sed -i "s/^atomic_bridge = \".*\"/atomic_bridge = \"$RESOURCE_ADDRESS\"/" "$MOVE_TOML_PATH"
sed -i "s/^moveth = \".*\"/moveth = \"$RESOURCE_ADDRESS\"/" "$MOVE_TOML_PATH"
sed -i "s/^master_minter = \".*\"/master_minter = \"$RESOURCE_ADDRESS\"/" "$MOVE_TOML_PATH"
sed -i "s/^minter = \".*\"/minter = \"$RESOURCE_ADDRESS\"/" "$MOVE_TOML_PATH"
sed -i "s/^admin = \".*\"/admin = \"$RESOURCE_ADDRESS\"/" "$MOVE_TOML_PATH"
sed -i "s/^origin_addr = \".*\"/origin_addr = \"$ADDRESS\"/" "$MOVE_TOML_PATH"
sed -i "s/^source_account = \".*\"/source_account = \"$ADDRESS\"/" "$MOVE_TOML_PATH"

echo "Move.toml updated with ADDRESS: $ADDRESS and RESOURCE_ADDRESS: $RESOURCE_ADDRESS"