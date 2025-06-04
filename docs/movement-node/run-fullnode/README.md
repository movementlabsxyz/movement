# Full Node

Full nodes are responsible for sending transactions, synchronizing with the DA-Sequencer, and executing streamed blocks. They also provide RPC and indexer gRPC services. This document provides instructions on how to set up a full node to sync with the DA-Sequencer.

## Hardware Recommendations

Running a follower node locally allows you to evaluate performance on a given network. If you are joining a high-load network, such as the Movement Testnet, we recommend the following:

- 32 CPU cores  
- 64 GB RAM  
- 2 TB SSD with at least 60K IOPS and 200 MiB/s throughput  

## Container Revision

The current container revision for installation is: `CONTAINER_REV=45244c0` githut commit:`45244c01812e765bff6f69bb14c26b77372298ed`

## Running a Movement Full Node

You can join any network as a Full Node by running a Movement Node using container tags that point to the latest commit hash, along with the [`fullnode`](../../../docker/compose/movement-full-node/docker-compose.fullnode.yml) overlay.

You can install the node software using the automated Docker procedure or by installing it manually.

## Docker Installation

Docker installation is done in two steps:

1. Install the Full Node software on an instance.  
2. Configure the node to sync with a specific network.

Three networks are available, and you can run a node for any of them.

The installation process is the same for all networks—the only differences are the configuration parameters. Be sure to use the correct ones.

### Full Node instance installation

An Ansible script is provided to deploy the required software to an instance. Use the appropriate command depending on your target network:

- [Devnet](ansible/devnet/README.md)  
- [Testnet](ansible/testnet/README.md)  
- [Mainnet](ansible/mainnet/README.md)

The username (`ubuntu` used in the script) is an example. You must define a user that allows the script to connect to instances listed in the inventory.  
All software will be installed under this user.

Once the Ansible script has completed, the software will be installed on the instance. You must then configure the node to begin syncing.

### Configure Full Node

Connect to the instance then Stop the node before configuring it:

```bash
sudo systemctl stop movement-fullnode.service
```

#### Specific update for follower node

If you were previously running a follower node (e.g., version 0.3.4), you need to update the systemd service definition.

1. Rename the service file:

```bash
sudo mv /etc/systemd/system/movement-full-follower.service /etc/systemd/system/movement-fullnode.service
```

2. Depending on your target network, replace the service file content with the appropriate template:

 * [Devnet](ansible/devnet/movement-fullnode.service.j2)
 * [Testnet](ansible/testnet/movement-fullnode.service.j2)
 * [Mainnet](ansible/mainnet/movement-fullnode.service.j2)

Replace `{{ user }}` with your user and the `{{ rev }}` with the container rev above.

3. Update the `$HOME/movement` github checkout commit the container revision commit (see above).

 ```bash
 cd $HOME/movement
 git checkout <Last container commit rev>
 ```

#### Update config

To set up the node configuration file, or update it (if you're starting from an existing installation), you need to run the migration script.

First set the DA-Sequencer connection URL depending on the network:

 * Devnet: `export MAPTOS_DA_SEQUENCER_CONNECTION_URL=https://da-sequencer.devnet.movementinfra.xyz`
 * Testnet: `export MAPTOS_DA_SEQUENCER_CONNECTION_URL=https://da-sequencer.testnet.movementinfra.xyz`
 * Mainnet: `export MAPTOS_DA_SEQUENCER_CONNECTION_URL=https://da-sequencer.mainnet.movementinfra.xyz`

Then run the setup/migration script:

```bash
$HOME/movement/docs/movement-node/run-fullnode/scripts/setup_migrate.sh
```

If you want the full node to send transactions, it must be registered with the Movement DA-Sequencer.
After executing the setup/migration script, a batch signing public key will be printed.
Note the key and send it to the Movement team so it can be added to the DA-Sequencer full node whitelist.

#### Sync the node

By default, syncing from height zero is disabled to prevent unintentionally streaming the entire blockchain from genesis.
The recommended method is to restore the node from the most recent snapshot using the appropriate script:

  * Devnet: `$HOME/movement/docs/movement-node/run-fullnode/scripts/devnet/restore.sh`
  * Testnet: `$HOME/movement/docs/movement-node/run-fullnode/scripts/testnet/restore.sh`
  * Mainnet: `$HOME/movement/docs/movement-node/run-fullnode/scripts/mainnet/restore.sh`

#### Start the node

After restoring the database, restart the node:

```bash
systemctl restart movement-fullnode.service
```

The node should now start and begin syncing.

### Verify the node is working

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

### state from Da verification failed
When there's a divergence between the main node state and the full node state, it is detected on the full node, and the node stops with an error containing the sentence: `state from Da verification failed`.

To recover, try restarting the node. If you get the same error, perform a restoration to restart from a correct state. After restoration, restart the node; it should then sync correctly.