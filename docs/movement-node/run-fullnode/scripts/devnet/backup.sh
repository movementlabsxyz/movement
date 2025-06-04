#!/bin/bash -e

# export DOT_MOVEMENT_PATH=/home/ssm-user/.movement
export DOT_MOVEMENT_PATH=$HOME/.movement
export AWS_REGION="us-west-2"
export RESTIC_PASSWORD="movebackup"
export RESTIC_HOST="devnet_fullnode"
export SYNC_BUCKET="movement-sync-devnet"

echo "Running docker compose backup"
/usr/bin/docker compose -f ./movement/docker/compose/movement-full-node/snapshot/docker-compose.backup.yml up --force-recreate
