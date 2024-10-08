# propose.sh

#!/bin/bash

# Initialize contract variable
contract=""
url=""

# Parse options using getopts
while getopts "c:u:" opt; do
  case $opt in
    c) contract="$OPTARG"
    ;;
    u) url="$OPTARG"
    ;;
    \?) echo "Invalid option: -$OPTARG" >&2
        exit 1
    ;;
  esac
done

echo "Contract: $contract"
echo "URL: $url"

# Ensure the contract flag is provided
if [ -z "$contract" ]; then
  echo "Error: -c flag for contract is required."
  exit 1
fi

# Ensure the url flag is provided
if [ -z "$url" ]; then
  echo "Error: -u flag for url is required."
  exit 1
fi

# Make the curl request and store the result in a variable
response=$(curl -s -X POST \
  -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' \
  $url)

# Extract the 'result' field using jq and store it in a variable
chain_id_hex=$(echo $response | jq -r '.result')

# Convert the hex chain ID to decimal
chain_id_dec=$(printf "%d\n" $chain_id_hex)

# Run the script to generate transaction data for the upgrade
echo "Generating transaction data to upgrade contract $contract"
nix develop --command bash -c "cd .. && forge script "./script/${contract}Deployer.s.sol" -vvvv --fork-url ${url}"

# Convert contract name to lowercase
lowercase_contract=$(echo "$contract" | tr '[:upper:]' '[:lower:]')

# Run the upgrader script
echo "Running upgrader/propose.ts"
npx tsx ./propose.ts -c "$lowercase_contract" -u "$url"
