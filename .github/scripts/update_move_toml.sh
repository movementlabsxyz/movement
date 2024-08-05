#!/bin/bash

MOVE_TOML_PATH="protocol-units/bridge/move-modules/Move.toml"

INIT_OUTPUT=$(aptos init)

echo "Aptos init output:"
echo "$INIT_OUTPUT"

ADDRESS=$(echo "$INIT_OUTPUT" | grep -oP 'Account 0x[a-f0-9]{64}' | head -n 1 | awk '{print $2}')

if [[ -z "$ADDRESS" ]]; then
    echo "Error: Failed to extract the Aptos account address."
    exit 1
fi

echo "Extracted Aptos Account Address: $ADDRESS"

sed -i "s/^atomic_bridge = \".*\"/atomic_bridge = \"$ADDRESS\"/" "$MOVE_TOML_PATH"
sed -i "s/^moveth = \".*\"/moveth = \"$ADDRESS\"/" "$MOVE_TOML_PATH"
sed -i "s/^master_minter = \".*\"/master_minter = \"$ADDRESS\"/" "$MOVE_TOML_PATH"
sed -i "s/^minter = \".*\"/minter = \"$ADDRESS\"/" "$MOVE_TOML_PATH"
sed -i "s/^admin = \".*\"/admin = \"$ADDRESS\"/" "$MOVE_TOML_PATH"

echo "Move.toml updated with address: $ADDRESS"