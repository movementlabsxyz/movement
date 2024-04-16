#!/bin/bash

# Specify the directory paths
data_dir="/opt/aptos/data"
genesis_dir="/opt/aptos/genesis"

# Check if the data directory exists
if [ -d "$data_dir" ]; then
    echo "Removing $data_dir directory..."
    sudo rm -rf "$data_dir"
    echo "$data_dir directory removed successfully."
else
    echo "$data_dir directory does not exist."
fi

# Check if the genesis directory exists
if [ -d "$genesis_dir" ]; then
    echo "Removing $genesis_dir directory..."
    sudo rm -rf "$genesis_dir"
    echo "$genesis_dir directory removed successfully."
else
    echo "$genesis_dir directory does not exist."
fi

echo "Cleanup completed."
