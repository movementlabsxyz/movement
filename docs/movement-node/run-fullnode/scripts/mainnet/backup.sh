#!/bin/bash -e

export DOT_MOVEMENT_PATH=$HOME/.movement
export AWS_REGION="us-west-2"
export RESTIC_PASSWORD="movebackup"
export RESTIC_HOST="mainnet_fullnode"
export SYNC_BUCKET="movement-sync-mainnet"

echo "Running docker compose backup"
/usr/bin/docker compose -f ./movement/docker/compose/movement-full-node/snapshot/docker-compose.backup.yml up --force-recreate
