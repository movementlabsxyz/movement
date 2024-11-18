# Movement Follower Node
The Movement Follower Node is a node type introduced for RPC providers from the work on the Open Network PR documented under [Movement Node Experimental](../../../../movement-node-experimental/). The follower node type runs executions and forwards transaction to block proposers included in the trusted validator set. Follower Nodes help the Movement Network to scale by providing increased transaction ingress capacity and horizontal scaling for queries over network state.

## Getting Started
We provide more comprehensive guides for deployment later in this document and elsewhere. However, it is easiest to gain quick familiarity with the Movement Follower Node by running it locally.

To do this, ensure you have `nix` installed. We recommend the Determinate Systems `nix` installation script. You can find it [here](https://determinate.systems/posts/determinate-nix-installer/).

```bash
curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix | sh -s -- install
```

After installing `nix`, clone the Movement repository and open the `nix-shell` environment.

```bash
git clone https://github.com/movementlabsxyz/movement
cd movement
nix develop
```

This should install all dependencies needed to work on the Movement Follower Node.

You can now either run the follower node natively or with our containers via the provided `just` commands.

First create, an environment file for the follower node. The example below is for the Movement Testnet. Comments are made on how to change the environment file for other networks.

```bash
CONTAINER_REV=<latest-commit-hash>
DOT_MOVEMENT_PATH=./.movement
MAPTOS_CHAIN_ID=250 # change this to the chain id of the network you are running
MOVEMENT_SYNC="follower::mtnet-l-sync-bucket-sync<=>{maptos,maptos-storage,movement-da-db}/**" # change to the sync bucket for the network you are running
M1_DA_LIGHT_NODE_CONNECTION_PROTOCOL=https
M1_DA_LIGHT_NODE_CONNECTION_HOSTNAME="m1-da-light-node.testnet.bardock.movementlabs.xyz" # changes this to the hostname of the m1_da_light_node_service on network you are running
M1_DA_LIGHT_NODE_CONNECTION_PORT=443
# you may need to provide AWS credentials for the Amazon SDK to properly interact with the sync bucket
# often this will be picked up appropriately if your environment is configured to use AWS
# the bucket has public read access so you may not need to provide credentials
AWS_ACCESS_KEY_ID=<your-access-key>
AWS_SECRET_ACCESS_KEY=<your-secret-access-key>
AWS_DEFAULT_REGION=us-west-1 # change this to match the region of the sync bucket
AWS_REGION=us-west-1 # change this to match the region of the sync bucket
```

To run natively you can use the following command:

```bash
source .env
just movement-full-node native build.setup.follower -t=false
```

To run with containers you can use the following command:

```bash
just movement-full-node docker-compose follower
```

To check on the status of the service under either runner, run:

```bash
curl http://localhost:30731/v1
```

You should see a `ledger_version` field CLOSE to the other values on the network, e.g., [https://aptos.testnet.bardock.movementlabs.xyz/v1](https://aptos.testnet.bardock.movementlabs.xyz/v1).

## Deployment and Advanced Usage
For deployment and advanced usage, we recommend you use our [provided Ansible scripts](../../ansible/follower-node/README.md).