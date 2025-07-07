#!/bin/bash -e

export DOT_MOVEMENT_PATH="$HOME/.movement"
export AWS_REGION="us-west-2"
export RESTIC_PASSWORD="movebackup"
export RESTIC_HOST="devnet_fullnode"
export SYNC_BUCKET="movement-sync-devnet"
export RESTIC_REPOSITORY="s3:s3.${AWS_REGION}.amazonaws.com/${SYNC_BUCKET}/restic_node_backup"

echo "Removing old Movement DB files..."

rm -rf "$DOT_MOVEMENT_PATH/maptos"
rm -rf "$DOT_MOVEMENT_PATH/maptos-storage"
rm -rf "$DOT_MOVEMENT_PATH/movement-da-db"

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
