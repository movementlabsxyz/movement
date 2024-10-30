#!/usr/bin/env bash

# Check if execute_move argument is set to true
EXECUTE_MOVE=$1
BRIDGE_OPERATOR_ADDRESS=$2

# Define the directory and file paths
MOVEMENT_DIR="./.movement"
CONFIG_FILE="$MOVEMENT_DIR/config.yaml"

NEW_ACCOUNT="0xA550C18"

# Ensure the correct number of arguments
if [ -z "$EXECUTE_MOVE" ] || [ -z "$BRIDGE_OPERATOR_ADDRESS" ]; then
  echo "Usage: $0 <execute_move> <bridge_operator_address>"
  echo "Where <execute_move> is either 'true' or 'false'"
  echo "And <bridge_operator_address> is the address of the bridge operator"
  exit 1
fi

if [ ! -d "$MOVEMENT_DIR" ]; then
  echo "Error: Directory $MOVEMENT_DIR not found."
  exit 1
fi

if [ ! -f "$CONFIG_FILE" ]; then
  echo "Error: File $CONFIG_FILE not found."
  exit 1
fi

# Use sed to update the account field in the config.yaml file
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS (BSD sed)
    sed -i '' "s/^    account: .*/    account: ${NEW_ACCOUNT}/" "$CONFIG_FILE"
else
    # Linux (GNU sed)
    sed -i "s/^    account: .*/    account: ${NEW_ACCOUNT}/" "$CONFIG_FILE"
fi

echo "Account field updated with value: ${NEW_ACCOUNT}"

# Execute the Move scripts if execute_move is true
if [ "$EXECUTE_MOVE" == "true" ]; then
  echo "Executing Move scripts..."
  movement move compile \
    --package-dir protocol-units/bridge/move-modules/
    
  # First script: enable_bridge_feature
  movement move run-script \
    --compiled-script-path protocol-units/bridge/move-modules/build/bridge-modules/bytecode_scripts/enable_bridge_feature.mv \
    --profile default \
    --assume-yes 2>&1 | tee enable_bridge_feature_output.log

  # Second script: store_mint_burn_caps
  movement move run-script \
    --compiled-script-path protocol-units/bridge/move-modules/build/bridge-modules/bytecode_scripts/store_mint_burn_caps.mv \
    --profile default \
    --assume-yes 2>&1 | tee store_mint_burn_caps_output.log

  # Third script: update_bridge_operator 
  movement move run-script \
    --compiled-script-path protocol-units/bridge/move-modules/build/bridge-modules/bytecode_scripts/update_bridge_operator.mv \
    --args address:${BRIDGE_OPERATOR_ADDRESS} \
    --profile default \
    --assume-yes 2>&1 | tee update_bridge_operator_output.log

  echo "Move scripts executed."
else
  echo "Skipping Move script execution."
fi
