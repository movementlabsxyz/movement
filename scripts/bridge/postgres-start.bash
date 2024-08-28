#!/usr/bin/env bash
set -e

# Set environment variables
POSTGRES_USER="postgres"
POSTGRES_PASSWORD="password"
POSTGRES_DB="postgres"
POSTGRES_DB_HOST="${POSTGRES_DB_HOST:-localhost}"
DOT_MOVEMENT_PATH="${DOT_MOVEMENT_PATH:-./.movement}"
MAPTOS_INDEXER_GRPC_LISTEN_HOSTNAME="${MAPTOS_INDEXER_GRPC_LISTEN_HOSTNAME:-localhost}"
INDEXER_PROCESSOR_POSTGRES_CONNECTION_STRING="postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@${POSTGRES_DB_HOST}:5432/${POSTGRES_DB}"

# Start PostgreSQL
echo "Starting PostgreSQL 15..."
pg_ctl -D /opt/homebrew/var/postgresql@15 -l logfile start

# Wait for PostgreSQL to start
echo "Waiting for PostgreSQL to be ready..."
until pg_isready -h "$POSTGRES_DB_HOST" -p 5432 -U "$POSTGRES_USER"; do
  sleep 1
done
echo "PostgreSQL is ready."
