#!/bin/bash -e

export DOT_MOVEMENT_PATH=$HOME/.movement
export AWS_REGION="us-west-2"
#export AWS_REGION="us-east-1"
export RESTIC_PASSWORD="movebackup"
export SYNC_BUCKET="movement-sync-devnet"
#export SYNC_BUCKET="phil-indexer-suzukadb"

#/usr/bin/docker compose -f ./movement/docker/compose/movement-full-node/snapshot/docker-compose.restore_restic.yml up --force-recreate
docker compose -f docker/compose/movement-full-node/snapshot/docker-compose.restore_restic.yml up --force-recreate
