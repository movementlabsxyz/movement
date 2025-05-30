#!/bin/bash -e

export DOT_MOVEMENT_PATH=$HOME/.movement
export AWS_REGION="us-west-2"
#export AWS_REGION="us-east-1"
export RESTIC_PASSWORD="movebackup"
export SYNC_BUCKET="movement-sync-testnet"

#restic -r s3:s3.us-west-2.amazonaws.com/${SYNC_BUCKET}/restic_node_backup init

/usr/bin/docker compose -f ./movement/docker/compose/movement-full-node/snapshot/docker-compose.save_and_push_restic.yml up --force-recreate
