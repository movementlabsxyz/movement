# Follower Node
Follower Nodes are nodes that are configured to sync with a Da-Sequencer and execute streamed block. It provides RPC and Tx grpc service too. This document provides instructions on how to set up a Follower Node to sync with the Da Sequencer.

## Hardware Recommendations
By running the a Follower Node locally, you will be able to gauge the performance on a given network. If you are joining a network with high load, like the Movement Testnet, we recommend the following:
- 32 cores
- 64 GB RAM
- 2 TB SSD w/ 60K IOPS and 200 MiB/s throughput

## Container rev

The current container rev for installation is:

CONTAINER_REV=d963665

## Running a Movement Node on Follower Node
You can join any sufficiently upgraded network as a Folloewr Node by running a Movement Node with container tags specified to latest commit hash on this branch, the [`follower`](../../../../../docker/compose/movement-full-node/docker-compose.follower.yml) overlay. 

For example, here's how a template for a systemd service file running the above via Docker Compose might look, where the template parameters are replaced with the appropriate values above:

```ini
[Unit]
Description=Movement Full Node
After=network.target

[Service]
Type=simple
User={{ user }}
WorkingDirectory=/home/{{ user }}/movement
Environment="DOT_MOVEMENT_PATH=/home/{{ user }}/.movement"
Environment="CONTAINER_REV={{ rev }}"
ExecStart=/usr/bin/docker compose --env-file .env -f /home/{{ user }}/movement/docker/compose/movement-full-node/docker-compose.fullnode.yml up --force-recreate --remove-orphans
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

An Ansible script to deploy the above systemd service is available [here for testnet](./testnet/movement-full-follower.yml) and [here for Mainnet](./mainnet/movement-full-follower.yml). An example usage with an ec2 inventory is below. You may also benefit from watching our tutorial [VIDEO](https://www.loom.com/share/59e6a31a08ef4260bdc9b082a3980f52).

```shell
ansible-playbook --inventory <your-inventory> \
    --user ubuntu  \
    --extra-vars "movement_container_version=${CONTAINER_REV}" \
    --extra-vars "user=ubuntu" \
    docs/movement-node/run/ansible/follower-node/movement-full-follower.yml \
    --private-key your-private-key.pem
```

The username (`ubuntu` in this example) is the remote user to connect to instances in the inventory.

This will set up the Movement Node to connect to sync with the Follower Node environment.

## After instance creation and restart
After the ansible script execution, the follower node should be installed but doesn't start. Syncing from height zero is not allowed by default or the configuration doesn't exist.

### Migration from version 0.3.4
Node prior version 0.3.4 must migrate their config. To do a file `migrate.sh` with the following lines.

```
#!/bin/bash -e

# Stop the node if needed.
systemctl stop  movement-full-follower.service

export DOT_MOVEMENT_PATH=$HOME/.movement
export CONTAINER_REV=d963665
export MAPTOS_CHAIN_ID=126

# Migrate teh config.
/usr/bin/docker compose --env-file $HOME/movement/.env -f $HOME/movement/docker/compose/movement-full-node/snapshot/docker-compose.migrate_from_0.3.4.yml up --force-recreate

```
Before executing, verify the `$HOME` variable is set correctly defined and point to the folder where the `.movement` folder is installed.
Execute the `migrate.sh` script.

### Restoration for a new installation

Before starting a new installation, the node DB must be restored first using the restoration script.

In the Home directory create a new script file call `restore.sh` and copy / paste this content using `nano` or `vi`.

For Mainnet:

```
#!/bin/bash -e

# Stop the node if needed.
systemctl stop  movement-full-follower.service

export DOT_MOVEMENT_PATH=$HOME/.movement
export CONTAINER_REV=d963665
export AWS_DEFAULT_REGION=us-west-1
export AWS_REGION=us-west-1
export MAPTOS_CHAIN_ID=126
export AWS_ACCESS_KEY_ID="<access key>"
export AWS_SECRET_ACCESS_KEY="<secret key>"

# Restore the DB.
/usr/bin/docker compose --env-file $HOME/movement/.env -f $HOME/movement/docker/compose/movement-full-node/snapshot/docker-compose.restore.yml up --force-recreate

# Start the node.
systemctl start  movement-full-follower.service

```


For Testnet:

```
#!/bin/bash -e

# Stop the node if needed.
systemctl stop  movement-full-follower.service

export DOT_MOVEMENT_PATH=$HOME/.movement
export CONTAINER_REV=d963665
export AWS_DEFAULT_REGION=us-west-1
export AWS_REGION=us-west-1
export MAPTOS_CHAIN_ID=250
export AWS_ACCESS_KEY_ID="<access key>"
export AWS_SECRET_ACCESS_KEY="<secret key>"

# Restore the DB.
/usr/bin/docker compose --env-file $HOME/movement/.env -f $HOME/movement/docker/compose/movement-full-node/snapshot/docker-compose.restore.yml up --force-recreate

# Start the node.
systemctl start  movement-full-follower.service

```

### Update from an existing installation

If you update from an existing installation, the setup script should update your configuration.

## Troubleshooting 

### S3 Bucket Error
If you encounter an error reported by the `setup` service for a reject bucket connection, ensure that you are able to access the bucket manually by getting objects via the AWS CLI. 

### Invalid Aptos State Error
If you encounter an error reported by the `setup` service for an invalid Aptos state, this likely because the sync has fetched state into an invalid location relative to your Docker Compose application. An Aptos state error will likely be the first one reported. However, it most likely indicates a corruption of all state Perform a hierarchy of checks:
1. Does the directory indicated by the `DOT_MOVEMENT_PATH` contain folders for `maptos`, `maptos-storage`, and `movement-da-db`?
2. Does each of these folders have successfully unarchived files? There should be no archives in these folders.
3. Is the host volume mounted correctly? Check the `volumes` section of your Docker Compose file.

### Forceful Writes
Most other bugs that emerged in early development should be handled by the forceful writes made by `syncador-v2`. However, this also means that if your application is not configured to allow for writes from the user running the Movement Full Follower servicer, then you will likely encounter errors. 
