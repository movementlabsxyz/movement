# Open Network
"Open Network" describes a series of environments used for testing and development of permissionless versions of the Movement Network. These environments are not intended for production use and are not yet deemed ready for our mainnet, testnet, nor devnet. 

## `l-monninger/open-network`
The `l-monninger/open-network` environment is the first public and permissionless environment for the Movement Network. It is a testing environment intended for use amongst partners and early adopters.

### Running a Movement Node on `l-monninger/open-network`
You can join the `l-monninger/open-network` environment by running a Movement Node with container tags specified to latest commit hash on this branch, the [`remote-no-celestia-light-node`](../../docker/compose/suzuka-full-node/docker-compose.remote-no-celestia-light-node.yml) overlay, and the following environment variables:

```bash
MOVEMENT_SYNC="l-monninger-open-network-suzuka-devnet-l-sync<=>{maptos,maptos-storage,suzuka-da-db}/**"
CELESTIA_RPC_CONNECTION_PROTOCOL=https
CELESTIA_RPC_CONNECTION_HOSTNAME=l-monninger.open-network.rpc.celestia.suzuka.devnet.movementlabs.xyz
CELESTIA_RPC_CONNECTION_PORT=443
CELESTIA_WEBSOCKET_CONNECTION_PROTOCOL=wss
CELESTIA_WEBSOCKET_CONNECTION_HOSTNAME=l-monninger.open-network.ws.celestia.suzuka.devnet.movementlabs.xyz
CELESTIA_WEBSOCKET_CONNECTION_PORT=443
```

For example, here's how a template for a systemd service file running the above via Docker Compose might look, where the template parameters are replaced with the appropriate values above:

```ini
[Unit]
Description=Suzuka Full Node
After=network.target

[Service]
Type=simple
User={{ user }}
WorkingDirectory=/home/{{ user }}/movement
Environment="DOT_MOVEMENT_PATH=/home/{{ user }}/.movement"
Environment="CONTAINER_REV={{ rev }}"
Environment="MOVEMENT_SYNC={{ movement_sync }}"
Environment"CELESTIA_RPC_CONNECTION_PROTOCOL={{ celestia_rpc_connection_protocol }}"
Environment="CELESTIA_RPC_CONNECTION_HOSTNAME={{ celestia_rpc_connection_hostname }}"
Environment="CELESTIA_RPC_CONNECTION_PORT={{ celestia_rpc_connection_port }}"
Environment="CELESTIA_WEBSOCKET_CONNECTION_PROTOCOL={{ celestia_websocket_connection_protocol }}"
Environment="CELESTIA_WEBSOCKET_CONNECTION_HOSTNAME={{ celestia_websocket_connection_hostname }}"
Environment="CELESTIA_WEBSOCKET_CONNECTION_PORT={{ celestia_websocket_connection_port }}"
ExecStart=/usr/bin/docker compose --env-file .env -f /home/{{ user }}/movement/docker/compose/suzuka-full-node/docker-compose.yml -f /home/{{ user }}/movement/docker/compose/suzuka-full-node/docker-compose.remote-no-celestia-light.yml -f /home/{{ user }}/movement/docker/compose/suzuka-full-node/docker-compose.faucet-replicas.yml up --force-recreate --remove-orphans
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

An Ansible script to deploy the above systemd service is available [here](./l-monninger/open-network/suzuka-full-node.yml).

This will set up the Movement Node to connect to sync with the `l-monninger/open-network` environment.