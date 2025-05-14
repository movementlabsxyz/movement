# Full Node
Full Nodes are nodes which start all the functions to send transactions, sync with the Da-Sequencer and execute streamed blocks. It provides RPC and indexer grpc services too. This document provides instructions on how to set up a Full Node to sync with the Da Sequencer.

## Hardware Recommendations
By running the a Follower Node locally, you will be able to gauge the performance on a given network. If you are joining a network with high load, like the Movement Testnet, we recommend the following:
- 32 cores
- 64 GB RAM
- 2 TB SSD w/ 60K IOPS and 200 MiB/s throughput

## Container rev

The current container rev for installation is:

CONTAINER_REV=289c0b8

## Running a Movement Node on Follower Node
You can join any network as a Full Node by running a Movement Node with container tags specified to latest commit hash on this branch, the [`fullnode`](../../../../../docker/compose/movement-full-node/docker-compose.fullnode.yml) overlay.

To install the node software, you can use the automated docker procedure or install manually.

### Docker installation
The docker installation is done in 2 steps:
 1. Install the software into an instance
 2. Configure them to sync to a specific network.

 Three networks are started and you can run a node for any of them.

 The process is the same for all networks, the differences are the parameters used. So take care to use the right ones.

#### Install Fullnode
An Ansible script is provided to deploy the needed software on an instance, execute this command depending on the network you need:

 * [Devnet](ansible/devnet/README.md)
 * [Testnet](ansible/testnet/README.md)
 * [Mainnet](ansible/mainnet/README.md)

 The username (`ubuntu` use in the scrpt) is an example of user to define so that the script is able to connect to instances in the inventory.
 All the software will be installed under this user.


The Ansible script has successfully installed the software into the instance. The node must be configured to sync its data.

#### Configure Fullnode

Stop the node using the command: `systemctl stop movement-fullnode.service`

1) Update config
The first step is to setup the node config file or update it if you start from an existing installation.

Define the da-sequencer connection url depending on the network:
 * Devnet: export MAPTOS_DA_SEQUENCER_CONNECTION_URL=https://da-sequencer.devnet.movementinfra.xyz
 * Testnet: export MAPTOS_DA_SEQUENCER_CONNECTION_URL=https://m1-da-light-node.testnet.bardock.movementnetwork.xyz
 * Mainnet: export MAPTOS_DA_SEQUENCER_CONNECTION_URL=https://m1-da-light-node.mainnet.movementnetwork.xyz

To setup/migrate run the script: `$HOME/movement/docs/movement-node/run-fullnode/scripts/setup_migrate.sh`

If you want to use the fullnode to send Tx, the node must be registered to the movement da-sequencer.
After execution setup/migration script execution, the batch signing public key generated during the setup is printed.
Send it to Movement team so that they can add it to the da-sequencer fullnode whitelist.

2) Sync the node
By default, syncing from height zero is not allowed to prevent inadvertently streaming all blocks from the origin. The easiest method is to restore the most recent snapshot of the node DB using the appropriate script for the network. Before running the script, update it with the access key and secret key provided by Movement.

  * Devnet: `$HOME/movement/docs/movement-node/run-fullnode/scripts/devnet/restore.sh`
  * Testnet: `$HOME/movement/docs/movement-node/run-fullnode/scripts/testnet/restore.sh`
  * Mainnet: `$HOME/movement/docs/movement-node/run-fullnode/scripts/mainnet/restore.sh`

After the Db restoration, the node can be restarted with the command: `systemctl restart movement-fullnode.service`

The node should start and sync.

## Verify the node is working

To verify that the full node has synced correctly, use these commands:
1) get local state: `curl 127.0.0.1:30731/v1`
2) get the network state:
 * Devnet:
 * Testnet: `curl https://testnet.bardock.movementnetwork.xyz/v1`
 * Mainnet: `curl https://mainnet.movementnetwork.xyz/v1`


Run both until you get the same `ledger_version` and `block_height` state in both. You can use `curl -Z url1 url2` to send both call at the same time.

If you don't get the same value after some time, or if the difference between your node and the main one doesn't change much, it means that the new full node is unable to sync.

Try restoring it again first. If the issue persists, contact Movement support.

## Troubleshooting 

### S3 Bucket Error
If you encounter an error reported by the `restoration` script for a reject bucket connection, ensure that you are able to access the bucket manually by getting objects via the AWS CLI.
Verify that you've updated the script with the right access key. 

### Restoration error: minimum throughput was specified at 1 B/s, but throughput of 0 B/s was observed
Add this env var to the restoration script:
```
export SYNCADOR_MAX_CONCURRENT_PULLS=2
```

### The node doesn't sync
If you can't get the same state as the main node, it means that the node hasn't executed the block the same way has the main node and it's DB must be restored.
Do a restoration and if it still doesn't sync contact Movement support.