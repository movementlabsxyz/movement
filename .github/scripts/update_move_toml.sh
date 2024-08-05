# Path to your Move.toml file
MOVE_TOML_PATH="protocol-units/bridge/move-modules/Move.toml"

# Initialize Aptos and capture output
INIT_OUTPUT=$(aptos init)

# Debugging: Print the output of aptos init
echo "Aptos init output:"
echo "$INIT_OUTPUT"

# Extract the address using grep
ADDRESS=$(echo "$INIT_OUTPUT" | grep -oP 'Account \K0x[a-f0-9]{64}')

# Check if the address was successfully extracted
if [[ -z "$ADDRESS" ]]; then
    echo "Error: Failed to extract the Aptos account address."
    exit 1
fi

# Update the Move.toml with the new address
sed -i "s/^atomic_bridge = \".*\"/atomic_bridge = \"$ADDRESS\"/" "$MOVE_TOML_PATH"
sed -i "s/^moveth = \".*\"/moveth = \"$ADDRESS\"/" "$MOVE_TOML_PATH"
sed -i "s/^master_minter = \".*\"/master_minter = \"$ADDRESS\"/" "$MOVE_TOML_PATH"
sed -i "s/^minter = \".*\"/minter = \"$ADDRESS\"/" "$MOVE_TOML_PATH"
sed -i "s/^admin = \".*\"/admin = \"$ADDRESS\"/" "$MOVE_TOML_PATH"

echo "Move.toml updated with address: $ADDRESS"