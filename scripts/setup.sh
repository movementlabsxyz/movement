#!/bin/bash

# Specify the directory paths
data_dir="/opt/aptos/data"
genesis_dir="/opt/aptos/genesis"

# Check if the data directory exists
if [ ! -d "$data_dir" ]; then
    # Create the data directory if it doesn't exist
    if mkdir -p "$data_dir" 2>/dev/null; then
        echo "Directory '$data_dir' created."
    else
        echo "Failed to create directory '$data_dir'. Please check permissions and try again with appropriate privileges (e.g., using sudo)."
        exit 1
    fi
else
    echo "Directory '$data_dir' already exists."
fi

# Check if the genesis directory exists
if [ ! -d "$genesis_dir" ]; then
    # Create the genesis directory if it doesn't exist
    if mkdir -p "$genesis_dir" 2>/dev/null; then
        echo "Directory '$genesis_dir' created."
    else
        echo "Failed to create directory '$genesis_dir'. Please check permissions and try again with appropriate privileges (e.g., using sudo)."
        exit 1
    fi
else
    echo "Directory '$genesis_dir' already exists."
fi

# Download the genesis.blob file
echo "Downloading genesis.blob..."
curl -s https://devnet.aptoslabs.com/genesis.blob -o "$genesis_dir/genesis.blob"
if [ $? -eq 0 ]; then
    echo "genesis.blob downloaded successfully."
else
    echo "Failed to download genesis.blob. Please check your internet connection and try again."
    exit 1
fi

# Download the waypoint.txt file
echo "Downloading waypoint.txt..."
curl -s https://devnet.aptoslabs.com/waypoint.txt -o "$genesis_dir/waypoint.txt"
if [ $? -eq 0 ]; then
    echo "waypoint.txt downloaded successfully."
else
    echo "Failed to download waypoint.txt. Please check your internet connection and try again."
    exit 1
fi

# Create the validator-identity.yaml file
echo "Creating validator-identity.yaml..."
cat > "$genesis_dir/validator-identity.yaml" <<EOL
---
account_address: 79ef25da72da0a9cc6fd2bb2d8f9621bb70028172b17dacc03a05e2b2f789e4a
account_key: "0xadc77158c83de10b1a57dc2f0f905d8c43de140ce7fb9ca1d07035b69b3143a3"
consensus_key: "0xb0bafb27c8b81e7464f62a72372db122527fdc7ed0c5baa92cbcb31f49e67ce1"
network_key: "0xa089baa63daf9cee3a619067548a41e3ace03934ffbc092885e061f65d035768"
EOL

echo "validator-identity.yaml created successfully."
