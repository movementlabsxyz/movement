# safeDeploy.sh

#!/bin/bash

# Initialize contract variable
contract=""
url=""
api_key=""

# Parse options using getopts
while getopts "c:u:k:" opt; do
  case $opt in
    c) contract="$OPTARG"
    ;;
    u) url="$OPTARG"
    ;;
    k) api_key="$OPTARG"
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

# Ensure the api_key flag is provided
if [ -z "$api_key" ]; then
  echo "Error: -k flag for etherscan api key is required."
  exit 1
fi

# Run the script to generate transaction data for the deployment
echo "Generating transaction data to deploy contract $contract"
forge script "../script/${contract}Deployer.s.sol" -vvvv --fork-url ${url} --broadcast --verify --etherscan-api-key ${api_key}

# Run the deployer script
echo "Running upgrader/safeDeploy.ts"
npx tsx ./safeDeploy.ts -u "$url"
