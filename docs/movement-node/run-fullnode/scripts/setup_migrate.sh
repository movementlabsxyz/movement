#!/bin/bash -e

# systemctl stop  movement-full-follower.service
export DOT_MOVEMENT_PATH=$HOME/.movement
export CONTAINER_REV=61a3506
export MAYBE_RUN_LOCAL="false"

/usr/bin/docker compose --env-file movement/.env -f movement/docker/compose/movement-full-node/docker-compose.fullnode_setup.yml up --force-recreate
