#!/bin/bash -e

export DOT_MOVEMENT_PATH=/.movement
export CONTAINER_REV=d9f8180-amd64
export HOME="$(pwd)"
export MVT_NODE_REST_URL="http://192.168.88.161:30731"

DOT_MOVEMENT_PATH="$(pwd)/.movement" cargo run -p movement-full-node -- admin l1-migration change-epoch

#/usr/bin/docker run -v $HOME/.movement:/.movement --rm ghcr.io/movementlabsxyz/movement-full-node:${CONTAINER_REV} admin l1-migration change-epoch

