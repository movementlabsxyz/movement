# Full Node

Full nodes are responsible for sending transactions, synchronizing with the DA-Sequencer, and executing streamed blocks. They also provide RPC and indexer gRPC services. This document provides instructions on how to set up a full node to sync with the DA-Sequencer.

## Hardware Recommendations

Running a follower node locally allows you to evaluate performance on a given network. If you are joining a high-load network, such as the Movement Testnet, we recommend the following:

- 32 CPU cores  
- 64 GB RAM  
- 2 TB SSD with at least 60K IOPS and 200 MiB/s throughput  

## Container Revision

The current container revision for installation is: `CONTAINER_REV=a349ae1-amd64` githut commit:`a349ae1ec13357b07e4ecf8a32c6cb50defed620`

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

If you create a new node add these environment variables:

 * Devnet: `export  MAPTOS_CHAIN_ID=27 && export MAPTOS_PRIVATE_KEY=random`
 * Testnet: `export  MAPTOS_CHAIN_ID=250 && export MAPTOS_PRIVATE_KEY=random`
 * Mainnet: `export  MAPTOS_CHAIN_ID=126 && export MAPTOS_PRIVATE_KEY=random`


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

### Error "State from Da verification failed"

When there's a divergence between the main node state and the full node state, it is detected on the full node, and the node stops with an error containing the sentence: `state from Da verification failed`.

To recover, try restarting the node. If you get the same error, perform a restoration to restart from a correct state. After restoration, restart the node; it should then sync correctly.

### Warn "DA Sequencer reject batch, can't send the batch."

The full node batch signer key is not white listed. Send it to movement so that to register it to the da-sequencer.

### The node is blocked and doesn't start

If the node start but stop with this logs:
```
Attempting to bind metrics server to 0.0.0.0:9464
Metrics server successfully bound to 0.0.0.0:9464
Metrics server listening on 0.0.0.0:9464
```
It means that the config.json file hasn't been configured. Run the setup_migration script (see Update config).

### Error "Send execution state to da sequencer failed"

Update config.json to set `propagate_execution_state` field to "propagate_execution_state": false and restart.

### Error "he node shouldn't send state to the DA, change the config propagate_execution_state to "propagate_execution_state": false and resta"

The maptos database is not the one expected by the node. Stop the node and do a restoration.

### Error "WARN movement_da_sequencer_client: DA sequencer Http2 connection url:... failed"

The da-sequencer connection url is badly configured. Edit the config.json field `connection_url` and set it with the value corresponding the connected network define in Update config chapter.

### Error "movement-restore-db  | unknown command "latest" for "restic"

The environment needed by the docker script or not defined or not detected by docker.

### Error "Error: Da Sync from height zero is not allowed."

The restoration hasn't been done or the database is not loaded correctly. Verify the DOT_MOVEMENT variable use to start the node is the same as the one to do the restoration.

### Example of a config file for mainnet
Sensitive data and some unused has been removed.

```
{
  "maptos_config": {
    "chain": {
      "maptos_chain_id": 126,
      "maptos_rest_listen_hostname": "0.0.0.0",
      "maptos_rest_listen_port": 30731,
      "maptos_private_key_signer_identifier": {
        "Local": {
          "private_key_hex_bytes": "..."
        }
      },
      "maptos_read_only": false,
      "enabled_pruning": false,
      "maptos_ledger_prune_window": 50000000,
      "maptos_epoch_snapshot_prune_window": 50000000,
      "maptos_state_merkle_prune_window": 100000,
      "maptos_db_path": "/.movement/maptos/126/.maptos",
      "genesis_timestamp_microseconds": 1600000000000,
      "genesis_block_hash_hex": "25112f5405bbc65b2542a67d94094f12f4d2e287025480efcdb6200c5fed8671",
      "known_framework_release_str": "elsa",
      "dont_increase_epoch_until_version": 0
    },
    "indexer": {
      "maptos_indexer_grpc_listen_hostname": "0.0.0.0",
      "maptos_indexer_grpc_listen_port": 30734,
      "maptos_indexer_grpc_inactivity_timeout": 60,
      "maptos_indexer_grpc_inactivity_ping_interval": 10,
      "maptos_indexer_grpc_healthcheck_hostname": "0.0.0.0",
      "maptos_indexer_grpc_healthcheck_port": 8084
    },
    "indexer_processor": {
      "postgres_connection_string": "postgresql://postgres:password@localhost:5432",
      "indexer_processor_auth_token": "auth_token"
    },
    "client": {
      "maptos_rest_connection_hostname": "0.0.0.0",
      "maptos_rest_connection_port": 30731,
      "maptos_faucet_rest_connection_hostname": "0.0.0.0",
      "maptos_faucet_rest_connection_port": 30732,
      "maptos_indexer_grpc_connection_hostname": "0.0.0.0",
      "maptos_indexer_grpc_connection_port": 30734
    },
    "faucet": {
      "maptos_rest_connection_hostname": "0.0.0.0",
      "maptos_rest_connection_port": 30731,
      "maptos_faucet_rest_listen_hostname": "0.0.0.0",
      "maptos_faucet_rest_listen_port": 30732
    },
    "fin": {
      "fin_rest_listen_hostname": "0.0.0.0",
      "fin_rest_listen_port": 30733
    },
    "load_shedding": {
      "max_transactions_in_flight": null,
      "batch_production_time": 2000
    },
    "mempool": {
      "sequence_number_ttl_ms": 180000,
      "gc_slot_duration_ms": 2000,
      "max_tx_per_batch": 8192,
      "max_batch_size": 1048576
    },
    "access_control": {
      "ingress_account_whitelist": null
    },
    "da_sequencer": {
      "connection_url": "https://da-sequencer.mainnet.movementinfra.xyz",
      "batch_signer_identifier": {
        "Local": {
          "private_key_hex_bytes": "..."
        }
      },
      "stream_heartbeat_interval_sec": 10,
      "propagate_execution_state": false
    }

  },
  "celestia_da_light_node_config": {
    "network": "Local",
    "appd": {
      "celestia_rpc_listen_hostname": "0.0.0.0",
      "celestia_rpc_listen_port": 26657,
      "celestia_websocket_connection_protocol": "ws",
      "celestia_websocket_connection_hostname": "0.0.0.0",
      "celestia_websocket_connection_port": 26658,
      "celestia_websocket_connection_path": "",
      "celestia_auth_token": null,
      "celestia_chain_id": "movement",
      "celestia_namespace": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAbW92ZW1lbnQ=",
      "celestia_path": null,
      "celestia_validator_address": null,
      "celestia_appd_use_replace_args": false,
      "celestia_appd_replace_args": []
    },
    "bridge": {
      "celestia_rpc_connection_protocol": "http",
      "celestia_rpc_connection_hostname": "0.0.0.0",
      "celestia_rpc_connection_port": 26657,
      "celestia_websocket_listen_hostname": "0.0.0.0",
      "celestia_websocket_listen_port": 26658,
      "celestia_bridge_path": null,
      "celestia_bridge_use_replace_args": false,
      "celestia_bridge_replace_args": []
    },
    "light": {
      "key_name": "movement_celestia_light",
      "node_store": null
    },
    "da_light_node": {
      "celestia_rpc_connection_protocol": "http",
      "celestia_rpc_connection_hostname": "0.0.0.0",
      "celestia_rpc_connection_port": 26657,
      "celestia_websocket_connection_hostname": "0.0.0.0",
      "celestia_websocket_connection_port": 26658,
      "celestia_websocket_connection_path": "",
      "movement_da_light_node_listen_hostname": "0.0.0.0",
      "movement_da_light_node_listen_port": 30730,
      "movement_da_light_node_connection_protocol": "http",
      "movement_da_light_node_connection_hostname": "0.0.0.0",
      "movement_da_light_node_connection_port": 30730,
      "movement_da_light_node_http1": false,
      "da_signers": {
        "signer_identifier": {
          "Local": {
            "private_key_hex_bytes": "..."
          }
        },
        "public_keys_hex": [
          "0433d4cc423c3799cf3213d2b9e0fdcf94cec93fd3c52582f0bdef143ae008b233e778ab201e7d85e92044901e40b40d3cc9be67ce7591932a07390df1cfd7e152"
        ]
      }
    },
    "celestia_force_new_chain": true,
    "memseq": {
      "sequencer_chain_id": "test",
      "sequencer_database_path": "/tmp/sequencer",
      "memseq_build_time": 500,
      "memseq_max_block_size": 2048
    },
    "da_light_node_is_initial": true,
    "initial_height": 0,
    "access_control": {
      "ingress_account_whitelist": null
    },
    "digest_store": {
      "digest_store_db_path": "/tmp/nix-shell.rSYGNN/digest_store_db"
    }
  },
  "mcr": {
    "eth_connection": {
      "eth_rpc_connection_protocol": "https",
      "eth_rpc_connection_hostname": "ethereum-holesky-rpc.publicnode.com",
      "eth_rpc_connection_port": 443,
      "eth_ws_connection_protocol": "ws",
      "eth_ws_connection_hostname": "ethereum-holesky-rpc.publicnode.com",
      "eth_ws_connection_port": 443,
      "eth_chain_id": 0
    },
    "settle": {
      "should_settle": false,
      "signer_identifier": {
        "Local": {
          "private_key_hex_bytes": "..."
        }
      },
      "mcr_contract_address": "0x5fc8d32690cc91d4c39d9d3abcbd16989f875707",
      "settlement_super_block_size": 1,
      "settlement_admin_mode": false
    },
    "transactions": {
      "gas_limit": 10000000000000000,
      "batch_timeout": 2000,
      "transaction_send_retries": 10
    },
    "maybe_run_local": true,
    "deploy": null,
    "testing": null
  },
  "da_db": {
    "da_db_path": "/.movement/movement-da-db",
    "start_sync_height": 0,
    "allow_sync_from_zero": false
  },
  "execution_extension": {
    "block_retry_count": 10,
    "block_retry_increment_microseconds": 5000
  },
  "syncing": {
    "movement_sync": null,
    "application_id": [
      26,
      43,
 ...
      242,
      3
    ],
    "syncer_id": [
      138,
      138,
...
      187,
      41
    ],
    "root_dir": "/.movement"
  }
}


```
