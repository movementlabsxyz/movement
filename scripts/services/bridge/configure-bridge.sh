#!/usr/bin/env bash
set -euo pipefail

# This script updates the 'account' field in a configuration file and optionally executes Move scripts.
# Usage:
#   ./script.sh <execute_move> <bridge_operator_address>
# where:
#   <execute_move> is either 'true' or 'false'
#   <bridge_operator_address> is the address of the bridge operator.

EXECUTE_MOVE="${1:-}"
BRIDGE_OPERATOR_ADDRESS="${2:-}"

MOVEMENT_DIR="./.movement"
CONFIG_FILE="${MOVEMENT_DIR}/config.yaml"
NEW_ACCOUNT="0xA550C18"

# Check if movement command is available
if ! command -v movement &>/dev/null; then
  echo "Error: 'movement' command not found in PATH."
  exit 1
fi

# Ensure the correct number and type of arguments
if [ -z "${EXECUTE_MOVE}" ] || [ -z "${BRIDGE_OPERATOR_ADDRESS}" ]; then
  echo "Usage: $0 <execute_move> <bridge_operator_address>"
  echo "  <execute_move> should be 'true' or 'false'"
  echo "  <bridge_operator_address> must be a valid bridge operator address"
  exit 1
fi

if [ ! -d "${MOVEMENT_DIR}" ]; then
  echo "Error: Directory '${MOVEMENT_DIR}' not found."
  exit 1
fi

if [ ! -f "${CONFIG_FILE}" ]; then
  echo "Error: File '${CONFIG_FILE}' not found."
  exit 1
fi

# Update the 'account' field in the config file
# Using a case statement for portability between macOS and Linux
case "$OSTYPE" in
  darwin*)
    # macOS (BSD sed)
    sed -i '' "s/^[[:space:]]*account:.*/    account: ${NEW_ACCOUNT}/" "${CONFIG_FILE}"
    ;;
  *)
    # Linux (GNU sed)
    sed -i "s/^[[:space:]]*account:.*/    account: ${NEW_ACCOUNT}/" "${CONFIG_FILE}"
    ;;
esac

echo "Successfully updated the account field to '${NEW_ACCOUNT}' in ${CONFIG_FILE}"

# Execute Move scripts if EXECUTE_MOVE is 'true'
if [ "${EXECUTE_MOVE}" = "true" ]; then
  echo "Executing Move scripts..."

  # Compile Move packages
  movement move compile \
    --package-dir protocol-units/bridge/move-modules/

  # Run the enable_bridge_feature script
  movement move run-script \
    --compiled-script-path protocol-units/bridge/move-modules/build/bridge-modules/bytecode_scripts/enable_bridge_feature.mv \
    --profile default \
    --assume-yes 2>&1 | tee enable_bridge_feature_output.log

  # Run the store_mint_burn_caps script
  movement move run-script \
    --compiled-script-path protocol-units/bridge/move-modules/build/bridge-modules/bytecode_scripts/store_mint_burn_caps.mv \
    --profile default \
    --assume-yes 2>&1 | tee store_mint_burn_caps_output.log

  # Run the update_bridge_operator script
  movement move run-script \
    --compiled-script-path protocol-units/bridge/move-modules/build/bridge-modules/bytecode_scripts/update_bridge_operator.mv \
    --args "address:${BRIDGE_OPERATOR_ADDRESS}" \
    --profile default \
    --assume-yes 2>&1 | tee update_bridge_operator_output.log

  echo "Move scripts executed successfully."
else
  echo "Skipping Move script execution as requested."
fi
