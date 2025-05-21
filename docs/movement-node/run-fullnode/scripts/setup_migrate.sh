#!/bin/bash -e

systemctl stop  movement-fullnode.service
export DOT_MOVEMENT_PATH=$HOME/.movement
export CONTAINER_REV=0e8ae41
export MAYBE_RUN_LOCAL="false"

/usr/bin/docker compose --env-file movement/.env -f movement/docker/compose/movement-full-node/docker-compose.fullnode_setup.yml up --force-recreate
