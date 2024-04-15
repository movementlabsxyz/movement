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
