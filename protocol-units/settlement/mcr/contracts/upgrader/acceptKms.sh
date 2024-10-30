# acceptKms.sh

#!/bin/bash

# Initialize contract variable
contract=""
transaction_hash=""
key_id=""

# Parse options using getopts
while getopts "c:t:k:" opt; do
  case $opt in
    c) contract="$OPTARG"
    ;;
    t) transaction_hash="$OPTARG"
    ;;
    k) key_id="$OPTARG"
    ;;
    \?) echo "Invalid option: -$OPTARG" >&2
        exit 1
    ;;
  esac
done

echo "Contract: $contract"
echo "Transaction Hash: $transaction_hash"
echo "Key ID: $key_id"

# Ensure the contract flag is provided
if [ -z "$contract" ]; then
  echo "Error: -c flag for contract is required."
  exit 1
fi

# Ensure the transaction_hash flag is provided
if [ -z "$transaction_hash" ]; then
  echo "Error: -t flag for transaction_hash is required."
  exit 1
fi

# Ensure the key_id flag is provided
if [ -z "$key_id" ]; then
  echo "Error: -k flag for key_id is required."
  exit 1
fi

# Convert contract name to lowercase
lowercase_contract=$(echo "$contract" | tr '[:upper:]' '[:lower:]')

# Run the upgrader script
echo "Running upgrader/acceptKms.ts"
npx tsx ./acceptKms.ts -c "$lowercase_contract" -t $transaction_hash -k $key_id
