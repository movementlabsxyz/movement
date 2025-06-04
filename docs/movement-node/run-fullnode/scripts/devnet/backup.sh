#!/bin/bash -e

# export DOT_MOVEMENT_PATH=/home/ssm-user/.movement
export DOT_MOVEMENT_PATH=$HOME/.movement
export AWS_REGION="us-west-2"
export RESTIC_PASSWORD="movebackup"
export RESTIC_HOST="devnet_fullnode"
export SYNC_BUCKET="movement-sync-devnet"

echo "Checking repository status..."
if docker run --rm \
  -v /root/.aws:/root/.aws \
  -e AWS_REGION=${AWS_REGION} \
  -e RESTIC_PASSWORD=${RESTIC_PASSWORD} \
  restic/restic \
  -r s3:s3.${AWS_REGION}.amazonaws.com/${SYNC_BUCKET}/restic_node_backup init 2>/dev/null; then
    echo "Repository initialized successfully"
else
    echo "Repository already exists"
fi

echo "Running docker compose backup"
/usr/bin/docker compose -f ./movement/docker/compose/movement-full-node/snapshot/docker-compose.backup.yml up --force-recreate
