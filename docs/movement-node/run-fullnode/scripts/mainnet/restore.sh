#!/bin/bash -e

export DOT_MOVEMENT_PATH=$HOME/.movement
export AWS_REGION="us-west-2"
export RESTIC_PASSWORD="movebackup"
export RESTIC_HOST="mainnet_fullnode"
export SYNC_BUCKET="movement-sync-mainnet"

# Remove old DB files
# Remove old DB files
echo "Remove Maptos DB files"
if [ -d "$DOT_MOVEMENT_PATH/maptos" ]; then
  rm -rf $DOT_MOVEMENT_PATH/maptos
fi
if [ -d "$DOT_MOVEMENT_PATH/maptos-storage" ]; then
  rm -rf $DOT_MOVEMENT_PATH/maptos-storage
fi
if [ -d "$DOT_MOVEMENT_PATH/movement-da-db" ]; then
  rm -rf $DOT_MOVEMENT_PATH/movement-da-db
fi

/usr/bin/docker compose -f ./movement/docker/compose/movement-full-node/snapshot/docker-compose.restore.yml up --force-recreate
