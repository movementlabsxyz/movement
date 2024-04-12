#!/bin/bash

# Specify the directory path
directory="/opt/aptos/data"

# Check if the directory exists
if [ ! -d "$directory" ]; then
  # Create the directory if it doesn't exist
  if mkdir -p "$directory" 2>/dev/null; then
    echo "Directory '$directory' created."
  else
    echo "Failed to create directory '$directory'. Please check permissions and try again with appropriate privileges (e.g., using sudo)."
    exit 1
  fi
else
  echo "Directory '$directory' already exists."
fi