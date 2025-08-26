#!/bin/bash -e

export DOT_MOVEMENT_PATH=$HOME/.movement
export CONTAINER_REV=21c256e

/usr/bin/docker run --rm ghcr.io/movementlabsxyz/movement-full-node:${CONTAINER_REV} admin l1-migration change-epoch