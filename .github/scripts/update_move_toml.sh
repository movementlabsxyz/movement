#!/bin/bash

MOVE_TOML_PATH="protocol-units/bridge/move-modules/Move.toml"

ADDRESS=$(aptos init | grep 'Account address:' | awk '{print $3}')

sed -i "s/^atomic_bridge = \".*\"/atomic_bridge = \"$ADDRESS\"/" "$MOVE_TOML_PATH"
sed -i "s/^moveth = \".*\"/moveth = \"$ADDRESS\"/" "$MOVE_TOML_PATH"
sed -i "s/^master_minter = \".*\"/master_minter = \"$ADDRESS\"/" "$MOVE_TMOL_PATH"
sed -i "s/^minter = \".*\"/minter = \"$ADDRESS\"/" "$MOVE_TOML_PATH"
sed -i "s/^admin = \".*\"/admin = \"$ADDRESS\"/" "$MOVE_TOML_PATH"

echo "Move.toml updated with address: $ADDRESS"