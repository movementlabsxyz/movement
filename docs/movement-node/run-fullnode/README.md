# Full Node

Full nodes are responsible for sending transactions, synchronizing with the DA-Sequencer, and executing streamed blocks. They also provide RPC and indexer gRPC services. This document provides instructions on how to set up a full node to sync with the DA-Sequencer.

## Hardware Recommendations

Running a follower node locally allows you to evaluate performance on a given network. If you are joining a high-load network, such as the Movement Testnet, we recommend the following:

- 32 CPU cores  
- 64 GB RAM  
- 2 TB SSD with at least 60K IOPS and 200 MiB/s throughput  

## Container Revision

The current container revision for installation is: `CONTAINER_REV=e5b696e` githut commit:`e5b696edaf153148b0cc6b29a8a512fe20fba554`

## Running a Movement Full Node

You can join any network as a Full Node by running a Movement Node using container tags that point to the latest commit hash on this branch, along with the [`fullnode`](../../../../../docker/compose/movement-full-node/docker-compose.fullnode.yml) overlay.

You can install the node software using the automated Docker procedure or by installing it manually.

### Docker Installation

Docker installation is done in two steps:

1. Install the software on an instance.  
2. Configure the node to sync with a specific network.

Three networks are available, and you can run a node for any of them.

The installation process is the same for all networks—the only differences are the configuration parameters. Be sure to use the correct ones.

#### Install Full Node

An Ansible script is provided to deploy the required software to an instance. Use the appropriate command depending on your target network:

- [Devnet](ansible/devnet/README.md)  
- [Testnet](ansible/testnet/README.md)  
- [Mainnet](ansible/mainnet/README.md)

The username (`ubuntu` used in the script) is an example. You must define a user that allows the script to connect to instances listed in the inventory.  
All software will be installed under this user.

Once the Ansible script has completed, the software will be installed on the instance. You must then configure the node to begin syncing.

#### Configure Full Node

Connect to the instance then Stop the node before configuring it:

```bash
sudo systemctl stop movement-fullnode.service
```

##### 1) Update config

The first step is to set up the node configuration file, or update it if you're starting from an existing installation.

Set the DA-Sequencer connection URL depending on the network:

 * Devnet: `export MAPTOS_DA_SEQUENCER_CONNECTION_URL=https://da-sequencer.devnet.movementinfra.xyz`
 * Testnet: `export MAPTOS_DA_SEQUENCER_CONNECTION_URL=https://m1-da-light-node.testnet.bardock.movementnetwork.xyz`
 * Mainnet: `export MAPTOS_DA_SEQUENCER_CONNECTION_URL=https://m1-da-light-node.mainnet.movementnetwork.xyz`

Run the setup/migration script:

```bash
$HOME/movement/docs/movement-node/run-fullnode/scripts/setup_migrate.sh
```

If you want the full node to send transactions, it must be registered with the Movement DA-Sequencer.
After executing the setup/migration script, a batch signing public key will be printed.
Send this key to the Movement team so it can be added to the DA-Sequencer full node whitelist.

##### 2) Specific update for follower node

If you were previously running a follower node (e.g., version 0.3.4), you need to update the systemd service definition.

Rename the service file:

```bash
sudo mv /etc/systemd/system/movement-full-follower.service /etc/systemd/system/movement-fullnode.service
```

Depending on your target network, replace the service file content with the appropriate template:

 * Devnet: `$HOME/movement/docs/movement-node/run-fullnode/ansible/devnet/movement-fullnode.service.j2`
 * Testnet: `$HOME/movement/docs/movement-node/run-fullnode/ansible/testnet/movement-fullnode.service.j2`
 * Mainnet: `$HOME/movement/docs/movement-node/run-fullnode/ansible/mainnet/movement-fullnode.service.j2`

 The `$HOME/movement` github checkout commit must be updated with the container revision commit (see above).

 ```bash
 cd $HOME/movement
 git checkout <Last container commit rev>
 ```

##### 3) Sync the node

By default, syncing from height zero is disabled to prevent unintentionally streaming the entire blockchain from genesis.
The recommended method is to restore the node from the most recent snapshot using the appropriate script:

  * Devnet: `$HOME/movement/docs/movement-node/run-fullnode/scripts/devnet/restore.sh`
  * Testnet: `$HOME/movement/docs/movement-node/run-fullnode/scripts/testnet/restore.sh`
  * Mainnet: `$HOME/movement/docs/movement-node/run-fullnode/scripts/mainnet/restore.sh`

After restoring the database, restart the node:

```bash
systemctl restart movement-fullnode.service
```

The node should now start and begin syncing.

## Verify the node is working

To check that the Full Node is syncing properly:

1. get local state: `curl 127.0.0.1:30731/v1`
2. get the network state:
 * Devnet: `curl https://full.devnet.movementinfra.xyz/v1`
 * Testnet: `curl https://testnet.bardock.movementnetwork.xyz/v1`
 * Mainnet: `curl https://mainnet.movementnetwork.xyz/v1`

Use both commands until `ledger_version` and `block_height` match. You can also use: `curl -Z url1 url2` to call both endpoints in parallel.

If values do not converge after some time, or the difference remains constant, the Full Node is likely out of sync.

Try restoring it again. If the issue persists, contact Movement support.

## Troubleshooting

### S3 Bucket Error

If you encounter an error from the `restoration` script indicating a rejected bucket connection, make sure you can manually access the bucket using the AWS CLI.  
Also verify that the script is using the correct AWS access key and secret.

### The node doesn't sync
If your node cannot reach the same state as the main node, it means it did not execute blocks correctly and its database must be restored.

Perform a restoration using the appropriate snapshot. If the issue persists after restoration, contact Movement support.

