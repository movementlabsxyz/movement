#!/bin/bash -e

export DOT_MOVEMENT_PATH=/home/ubuntu/.movement
export CONTAINER_REV="04c47fa"
export AWS_DEFAULT_REGION=us-west-2
export AWS_REGION=us-west-2
export MAPTOS_CHAIN_ID=126
export SYNC_PATTERN="{default_signer_address_whitelist,maptos,maptos-storage,movement-da-db}/**"
export SYNC_BUCKET="move-main-rec-l-sb-sync"
export SYNC_ARCHIVE="0.tar.gz"

/usr/bin/docker compose --env-file movement/.env -f /home/ubuntu/movement/docker/compose/movement-full-node/snapshot/docker-compose.save_and_push.yml up --force-recreate
