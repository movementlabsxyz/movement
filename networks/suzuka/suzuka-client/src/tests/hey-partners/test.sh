#!/bin/sh

FULLNODE="http://localhost:30731"
FAUCET="http://localhost:30732"
PATH_TO_REPO="."

ls

cd src/tests/hey-partners

# Initializes an account if keys are not present
initialize_output=$(echo -ne '\n' | aptos init --network custom --rest-url $FULLNODE --faucet-url $FAUCET --assume-yes)

CONFIG_FILE=".aptos/config.yaml"

if [ ! -f "$CONFIG_FILE" ]; then
  echo "Initialization failed. Config file not found."
  exit 1
fi

aptos move compile

PrivateKey=$(grep 'private_key:' "$CONFIG_FILE" | awk -F': ' '{print $2}' | tr -d '"')

# Lookup the SwapDeployer address
lookup_address_output=$(aptos account lookup-address)
echo "Lookup Address Output: $lookup_address_output"
SwapDeployer=0x$(echo "$lookup_address_output" | grep -o '"Result": "[0-9a-fA-F]\{64\}"' | sed 's/"Result": "\(.*\)"/\1/')
if [ -z "$SwapDeployer" ]; then
  echo "SwapDeployer extraction failed."
  exit 1
fi

# Lookup the ResourceAccountDeployer address test IS expected to fail as long as we can retrieve the account address anyway
 test_resource_account_output=$(aptos move test --package-dir "$PATH_TO_REPO/Swap/" \
--filter test_resource_account --named-addresses SwapDeployer=$SwapDeployer,uq64x64=$SwapDeployer,u256=$SwapDeployer,ResourceAccountDeployer=$SwapDeployer)
echo "Test Resource Account Output: $test_resource_account_output"
ResourceAccountDeployer=$(echo "$test_resource_account_output" | grep -o '\[debug\] @[^\s]*' | sed 's/\[debug\] @\(.*\)/\1/')
if [ -z "$ResourceAccountDeployer" ]; then
  echo "ResourceAccountDeployer extraction failed."
  exit 1
fi

# Save variable to .env file for SDK tests
add_or_update_env() {
    local key=$1
    local value=$2
    local file=".env"
    if grep -q "^$key=" "$file"; then
        # Update the existing key with the new value
        if [[ "$OSTYPE" == "darwin"* ]]; then
            # macOS
            sed -i '' "s/^$key=.*/$key=$value/" "$file"
        else
            # Linux and other Unix-like systems
            sed -i "s/^$key=.*/$key=$value/" "$file"
        fi
    else
        # Add the key-value pair if it doesn't exist
        echo "$key=$value" >> "$file"
    fi
}

add_or_update_env "SWAP_DEPLOYER" $SwapDeployer
add_or_update_env "RESOURCE_ACCOUNT_DEPLOYER" $ResourceAccountDeployer
add_or_update_env "PRIVATE_KEY" $PrivateKey
add_or_update_env "FULLNODE" $FULLNODE
add_or_update_env "FAUCET" $FAUCET

# publish 
echo "Publish uq64x64"
aptos move publish --package-dir $PATH_TO_REPO/uq64x64/ --assume-yes --named-addresses uq64x64=$SwapDeployer 
echo "Publish u256"
aptos move publish --package-dir $PATH_TO_REPO/u256/ --assume-yes --named-addresses u256=$SwapDeployer 
echo "Publish TestCoin"
aptos move publish --package-dir $PATH_TO_REPO/TestCoin/ --assume-yes --named-addresses SwapDeployer=$SwapDeployer 
echo "Publish Faucet"
aptos move publish --package-dir $PATH_TO_REPO/Faucet/ --assume-yes --named-addresses SwapDeployer=$SwapDeployer
echo "Publish Resource Account"
aptos move publish --package-dir $PATH_TO_REPO/LPResourceAccount/ --assume-yes --named-addresses SwapDeployer=$SwapDeployer
# create resource account & publish LPCoin
# use this command to compile LPCoin
aptos move compile --package-dir $PATH_TO_REPO/LPCoin/ --save-metadata --named-addresses ResourceAccountDeployer=$ResourceAccountDeployer
# get the first arg
arg1=$(hexdump -ve '1/1 "%02x"' $PATH_TO_REPO/LPCoin/build/LPCoin/package-metadata.bcs)
# get the second arg
arg2=$(hexdump -ve '1/1 "%02x"' $PATH_TO_REPO/LPCoin/build/LPCoin/bytecode_modules/LPCoinV1.mv)
# This command is to publish LPCoin contract, using ResourceAccountDeployer address. Note: replace two args with the above two hex
echo "Initialize LPAccount"
aptos move run --function-id ${SwapDeployer}::LPResourceAccount::initialize_lp_account \
--args hex:$arg1 hex:$arg2 --assume-yes

echo "Publishing MovementSwap"
aptos move publish --package-dir $PATH_TO_REPO/Swap/ --assume-yes --named-addresses uq64x64=$SwapDeployer,u256=$SwapDeployer,SwapDeployer=$SwapDeployer,ResourceAccountDeployer=$ResourceAccountDeployer

# admin steps
# TestCoinsV1
echo "Initialize TestCoinsV1"
aptos move run --function-id ${SwapDeployer}::TestCoinsV1::initialize --assume-yes
echo "Mint USDT TestCoinsV1"
aptos move run --function-id ${SwapDeployer}::TestCoinsV1::mint_coin \
--args address:${SwapDeployer} u64:20000000000000000 \
--type-args ${SwapDeployer}::TestCoinsV1::USDT --assume-yes
echo "Mint BTC TestCoinsV1"
aptos move run --function-id ${SwapDeployer}::TestCoinsV1::mint_coin \
--args address:${SwapDeployer} u64:2000000000000 \
--type-args ${SwapDeployer}::TestCoinsV1::BTC --assume-yes

# FaucetV1
echo "Create USDT FaucetV1"
aptos move run --function-id ${SwapDeployer}::FaucetV1::create_faucet \
--args u64:10000000000000000 u64:1000000000 u64:3600 \
--type-args ${SwapDeployer}::TestCoinsV1::USDT --assume-yes
echo "Create BTC FaucetV1"
aptos move run --function-id ${SwapDeployer}::FaucetV1::create_faucet \
--args u64:1000000000000 u64:10000000 u64:3600 \
--type-args ${SwapDeployer}::TestCoinsV1::BTC --assume-yes

# AnimeSwapPool
echo "add USDT:MOVE pair"
aptos move run --function-id ${SwapDeployer}::AnimeSwapPoolV1::add_liquidity_entry \
--args u64:10000000000 u64:100000000 u64:1 u64:1 \
--type-args ${SwapDeployer}::TestCoinsV1::USDT 0x1::aptos_coin::AptosCoin --assume-yes
echo "add BTC:MOVE pair"
aptos move run --function-id ${SwapDeployer}::AnimeSwapPoolV1::add_liquidity_entry \
--args u64:10000000 u64:100000000 u64:1 u64:1 \
--type-args ${SwapDeployer}::TestCoinsV1::BTC 0x1::aptos_coin::AptosCoin --assume-yes
echo "add BTC:USDT pair"
aptos move run --function-id ${SwapDeployer}::AnimeSwapPoolV1::add_liquidity_entry \
--args u64:100000000 u64:100000000000 u64:1 u64:1 \
--type-args ${SwapDeployer}::TestCoinsV1::BTC ${SwapDeployer}::TestCoinsV1::USDT --assume-yes

echo "Finished Admin Functions"
# user
# fund
echo "Request USDT"
aptos move run --function-id ${SwapDeployer}::FaucetV1::request \
--args address:${SwapDeployer} \
--type-args ${SwapDeployer}::TestCoinsV1::USDT --assume-yes
echo "Request BTC"
aptos move run --function-id ${SwapDeployer}::FaucetV1::request \
--args address:${SwapDeployer} \
--type-args ${SwapDeployer}::TestCoinsV1::BTC --assume-yes
# swap (type args shows the swap direction, in this example, swap BTC to APT)
echo "Swap exact BTC for MOVE"
aptos move run --function-id ${SwapDeployer}::AnimeSwapPoolV1::swap_exact_coins_for_coins_entry \
--args u64:100 u64:1 \
--type-args ${SwapDeployer}::TestCoinsV1::BTC 0x1::aptos_coin::AptosCoin --assume-yes
# swap
echo "Swap BTC for exact MOVE"
aptos move run --function-id ${SwapDeployer}::AnimeSwapPoolV1::swap_coins_for_exact_coins_entry \
--args u64:100 u64:1000000000 \
--type-args ${SwapDeployer}::TestCoinsV1::BTC 0x1::aptos_coin::AptosCoin --assume-yes
# multiple pair swap (this example, swap 100 BTC->APT->USDT)
echo "Swap BTC for USDT"
aptos move run --function-id ${SwapDeployer}::AnimeSwapPoolV1::swap_exact_coins_for_coins_2_pair_entry \
--args u64:100 u64:1 \
--type-args ${SwapDeployer}::TestCoinsV1::BTC 0x1::aptos_coin::AptosCoin ${SwapDeployer}::TestCoinsV1::USDT --assume-yes
# add lp (if pair not exist, will auto create lp first)
echo "Add LP for BTC:MOVE"
aptos move run --function-id ${SwapDeployer}::AnimeSwapPoolV1::add_liquidity_entry \
--args u64:1000 u64:10000 u64:1 u64:1 \
--type-args ${SwapDeployer}::TestCoinsV1::BTC 0x1::aptos_coin::AptosCoin --assume-yes
echo "Remove LP from BTC:MOVE"
aptos move run --function-id ${SwapDeployer}::AnimeSwapPoolV1::remove_liquidity_entry \
--args u64:1000 u64:1 u64:1 \
--type-args ${SwapDeployer}::TestCoinsV1::BTC 0x1::aptos_coin::AptosCoin --assume-yes

# Admin cmd example
echo "Set dao fee"
aptos move run --function-id ${SwapDeployer}::AnimeSwapPoolV1::set_dao_fee_to \
--args address:${SwapDeployer} --assume-yes
echo "Set admin address"
aptos move run --function-id ${SwapDeployer}::AnimeSwapPoolV1::set_admin_address \
--args address:${SwapDeployer} --assume-yes
echo "set dao fee"
aptos move run --function-id ${SwapDeployer}::AnimeSwapPoolV1::set_dao_fee \
--args u64:5
echo "set swap fee"
aptos move run --function-id ${SwapDeployer}::AnimeSwapPoolV1::set_swap_fee \
--args u64:30 --assume-yes
echo "withdraw dao fee"
aptos move run --function-id ${SwapDeployer}::AnimeSwapPoolV1::withdraw_dao_fee \
--type-args ${SwapDeployer}::TestCoinsV1::BTC ${SwapDeployer}::TestCoinsV1::USDT --assume-yes
echo "pause"
aptos move run --function-id ${SwapDeployer}::AnimeSwapPoolV1::pause --assume-yes
echo "unpause"
aptos move run --function-id ${SwapDeployer}::AnimeSwapPoolV1::unpause --assume-yes

echo "Finished User Functions"

# Run SDK tests
echo "Running Typescript SDK tests"
npx ts-node ./tests/typescript-sdk/main.test.ts --yes