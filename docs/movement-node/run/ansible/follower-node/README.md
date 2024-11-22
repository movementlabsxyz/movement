# Follower Node
Follower Nodes are nodes that are configured to sync with a Leader Node and are used to test the Movement Testnet. This document provides instructions on how to set up a Follower Node to sync with a Leader Node.

## Hardware Recommendations
By running the a Follower Node locally, you will be able to gauge the performance on a given network. If you are joining a network with high load, like the Movement Testnet, we recommend the following:
- 32 cores
- 64 GB RAM
- 2 TB SSD w/ 60K IOPS and 200 MiB/s throughput

## Running a Movement Node on Follower Node
You can join any sufficiently upgraded network as a Folloewr Node by running a Movement Node with container tags specified to latest commit hash on this branch, the [`follower`](../../../../../docker/compose/movement-full-node/docker-compose.follower.yml) overlay. 

**Note**: the scripts provided herein have hardcoded constants for the Movement Testnet. You will need to change these to match the environment you are running on.

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
Environment="MOVEMENT_SYNC={{ movement_sync }}"
Environment="MOVEMENT_DA_LIGHT_NODE_CONNECTION_HOSTNAME={{ movement_da_light_node_connection_hostname }}"
ExecStart=/usr/bin/docker compose --env-file .env -f /home/{{ user }}/movement/docker/compose/movement-full-node/docker-compose.yml -f /home/{{ user }}/movement/docker/compose/movement-full-node/docker-compose.follower.yml up --force-recreate --remove-orphans
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

An Ansible script to deploy the above systemd service is available [here](./movement-full-follower.yml). An example usage with an ec2 inventory is below. You may also benefit from watching our tutorial [VIDEO](https://www.loom.com/share/59e6a31a08ef4260bdc9b082a3980f52).

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

For a basic check on syncing, assert that there is a `0.tar.gz` file in the `~/.movement` directory. This file is unarchived into the same directory when syncing. If you see it, that indicates that the syncing resource was fetched. It is not rearchived itself.

If you do not see the `0.tar.gz` that could indicate an issue with sync. See the troubleshooting steps below.

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
