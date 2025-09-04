#!/bin/bash -e

export DOT_MOVEMENT_PATH=/.movement
export CONTAINER_REV=63d8719-amd64
export HOME="$(pwd)"
export MVT_NODE_REST_URL="http://movement-full-node:30731"

/usr/bin/docker run -e MVT_NODE_REST_URL=$MVT_NODE_REST_URL -e DOT_MOVEMENT_PATH=$DOT_MOVEMENT_PATH -v $HOME/.movement:/.movement --rm ghcr.io/movementlabsxyz/movement-full-node:${CONTAINER_REV} admin l1-migration change-epoch

