#!/bin/bash -e

# Find the root of the repo (3 levels up from this script)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../../../" && pwd)"

export DOT_MOVEMENT_PATH="$HOME/.movement"
export AWS_REGION="us-west-2"
export RESTIC_PASSWORD="movebackup"
export RESTIC_HOST="mainnet_fullnode"
export SYNC_BUCKET="movement-sync-mainnet"

echo "Remove Maptos DB files"

rm -rf "$DOT_MOVEMENT_PATH/maptos"
rm -rf "$DOT_MOVEMENT_PATH/maptos-storage"
rm -rf "$DOT_MOVEMENT_PATH/movement-da-db"

# Use absolute path to docker-compose file
docker compose -f "$REPO_ROOT/docker/compose/movement-full-node/snapshot/docker-compose.restore.yml" up --force-recreate
