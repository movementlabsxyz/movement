#!/bin/bash -e

systemctl stop  movement-fullnode.service
export DOT_MOVEMENT_PATH=$HOME/.movement
export CONTAINER_REV="fa0f19b"
export AWS_REGION=us-west-2
export AWS_BUCKET_ANONYMOUS_ACCESS=true
export MAPTOS_CHAIN_ID=250
export SYNC_PATTERN="{default_signer_address_whitelist,maptos,maptos-storage,movement-da-db}/**"
export SYNC_BUCKET="movement-sync-testnet"

/usr/bin/docker compose --env-file movement/.env -f ./movement/docker/compose/movement-full-node/snapshot/docker-compose.restore.yml up --force-recreate
