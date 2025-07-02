#!/bin/bash -e

export DOT_MOVEMENT_PATH="$HOME/.movement"
export AWS_REGION="us-west-2"
export RESTIC_PASSWORD="movebackup"
export RESTIC_HOST="testnet_fullnode"
export SYNC_BUCKET="movement-sync-testnet"
export RESTIC_REPOSITORY="s3:s3.${AWS_REGION}.amazonaws.com/${SYNC_BUCKET}/restic_node_backup"

# Remove old DB files
echo "Removing Maptos DB files"
if [ -d "$DOT_MOVEMENT_PATH/maptos" ]; then
  rm -rf "$DOT_MOVEMENT_PATH/maptos"
fi
if [ -d "$DOT_MOVEMENT_PATH/maptos-storage" ]; then
  rm -rf "$DOT_MOVEMENT_PATH/maptos-storage"
fi
if [ -d "$DOT_MOVEMENT_PATH/movement-da-db" ]; then
  rm -rf "$DOT_MOVEMENT_PATH/movement-da-db"
fi

echo "Restoring latest snapshot from Restic..."

restic \
  --no-lock \
  -r "s3:s3.${AWS_REGION}.amazonaws.com/${SYNC_BUCKET}/restic_node_backup" \
  --host "$RESTIC_HOST" \
  restore latest \
  --target "$DOT_MOVEMENT_PATH" \
  --include "/.movement/maptos" \
  --include "/.movement/maptos-storage" \
  --include "/.movement/movement-da-db" \
  --include "/.movement/default_signer_address_whitelist" \
  -o s3.unsafe-anonymous-auth=true

echo "Restore complete."
