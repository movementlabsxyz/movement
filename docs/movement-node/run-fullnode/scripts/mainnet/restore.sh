#!/bin/bash -e

export DOT_MOVEMENT_PATH=$HOME/.movement
export AWS_REGION="us-west-2"
export RESTIC_PASSWORD="movebackup"
export RESTIC_HOST="mainnet_fullnode"
export SYNC_BUCKET="movement-sync-mainnet"

# Remove old DB files
rm -R $DOT_MOVEMENT_PATH/maptos $DOT_MOVEMENT_PATH/maptos-storage $DOT_MOVEMENT_PATH/movement-da-db

/usr/bin/docker compose -f ./movement/docker/compose/movement-full-node/snapshot/docker-compose.restore.yml up --force-recreate