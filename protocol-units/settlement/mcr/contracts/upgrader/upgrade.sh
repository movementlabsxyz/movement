#!/bin/bash

# Initialize contract variable
contract=""

# Parse options using getopts
while getopts "c:" opt; do
  case $opt in
    c) contract="$OPTARG"
    ;;
    \?) echo "Invalid option: -$OPTARG" >&2
        exit 1
    ;;
  esac
done

# Ensure the contract flag is provided
if [ -z "$contract" ]; then
  echo "Error: -c flag for contract is required."
  exit 1
fi

# Run the script to generate transaction data for the upgrade
echo "Generating transaction data to upgrade contract $contract"
nix develop --command bash -c "cd .. && forge script "./script/${contract}Deployer.s.sol" -vvvv --fork-url https://eth-sepolia.api.onfinality.io/public"

cd upgrader
# Convert contract name to lowercase
lowercase_contract=$(echo "$contract" | tr '[:upper:]' '[:lower:]')

# Run the upgrader script
echo "Running upgrader/index.ts"
npx tsx  ./index.ts -c "$lowercase_contract"
