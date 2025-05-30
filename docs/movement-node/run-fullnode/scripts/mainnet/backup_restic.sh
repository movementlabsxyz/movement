#!/bin/bash -e

export DOT_MOVEMENT_PATH=$HOME/.movement
export AWS_REGION="us-west-2"
#export AWS_REGION="us-east-1"
export RESTIC_PASSWORD="movebackup"
export SYNC_BUCKET="movement-sync-devnet"

/usr/bin/docker compose -f ./movement/docker/compose/movement-full-node/snapshot/docker-compose.save_and_push_restic.yml up --force-recreate
