# Run a Movement Node
- [General Full Node Guide](./README.md) (this document)
- [Follower Node](./follower-node/README.md)

We recommend that you run a movement node using containers to leverage the portability and reproducibility properties that come with containers.


## Prerequisites
1. Ubuntu 22.04 amd64
2. [Docker](https://docs.docker.com/engine/install/ubuntu/)
3. [Docker compose](https://docs.docker.com/compose/install/linux/)
4. Make sure you are logged in as the user who wants to run the node

## Run the movement node as an RPC provider


1. Clone the movement repo.

_The paths below are not mandatory, they are presented for a better understanding 
of the setup. If you would like to follow the steps below you will need to use them._

```bash
# From /home/${USER}
git clone https://github.com/movementlabsxyz/movement.git
cd movement
```

2. Create movement configuration directory and environment file.
At the moment the latest version of the movement network is named "Movement".
This will change in the future. Please check "Naming Conventions Latest" section in the
README.md file at the git root of this repo.

We use [just](https://github.com/casey/just?tab=readme-ov-file#installation) command
to instrument deployments. This tool reads `.env` files.
The configuration file for movement node is stored in `GIT_ROOT/.env`

We recommend to use the latest commit of the "main" branch:

```bash
GIT_ROOT=$(git rev-parse --show-toplevel)
MOVEMENT_ENV_FILE="${GIT_ROOT}/.env"
[[ -n "${GIT_ROOT}" ]] && touch "${MOVEMENT_ENV_FILE}"
mkdir -p .movement
```

3. Set the movement container version.
```bash
CONTAINER_REV=$(git rev-parse HEAD)
[[ -n "${CONTAINER_REV}" ]] \
  && export CONTAINER_REV=${CONTAINER_REV} \
  && echo "REV=${CONTAINER_REV}" >> "${MOVEMENT_ENV_FILE}"
echo "INFO: movement version is"
cat ${MOVEMENT_ENV_FILE}
```


4. Pull the container images. For this you need to make sure the movement config dir
exists.
```bash
DOT_MOVEMENT_PATH="~/.movement"
export DOT_MOVEMENT_PATH
echo "DOT_MOVEMENT_PATH=${DOT_MOVEMENT_PATH}" >> "${MOVEMENT_ENV_FILE}"

mkdir -p "${DOT_MOVEMENT_PATH}"
docker compose \
        -f docker/compose/movement-full-node/docker-compose.yml \
        -f docker/compose/movement-full-node/docker-compose.setup-local.yml \
        -f docker/compose/movement-full-node/docker-compose.celestia-local.yml \
        pull
```

5. Make sure that the containers image tag match the desired container version.
```bash
cat "${MOVEMENT_ENV_FILE}"
CONTAINER_REV=e6cb8e287cb837af6e61451f2ff405047dd285c9
DOT_MOVEMENT_PATH=/home/ubuntu/.movement

docker images | { head -1 ; grep movementlabsxyz ; }
REPOSITORY                                                 TAG                                        IMAGE ID       CREATED         SIZE
ghcr.io/movementlabsxyz/movement-full-node                   e6cb8e287cb837af6e61451f2ff405047dd285c9   f75f89ee0bda   3 days ago      131MB
ghcr.io/movementlabsxyz/movement-faucet-service              e6cb8e287cb837af6e61451f2ff405047dd285c9   a4dbed3f59b0   3 days ago      98.4MB
ghcr.io/movementlabsxyz/movement-full-node-setup             e6cb8e287cb837af6e61451f2ff405047dd285c9   5314611ab11a   3 days ago      244MB
ghcr.io/movementlabsxyz/movement-celestia-appd     e6cb8e287cb837af6e61451f2ff405047dd285c9   f23edec3d6d5   3 days ago      243MB
ghcr.io/movementlabsxyz/movement-celestia-da-light-node                   e6cb8e287cb837af6e61451f2ff405047dd285c9   31bee301f83c   3 days ago      90.5MB
ghcr.io/movementlabsxyz/movement-celestia-bridge   e6cb8e287cb837af6e61451f2ff405047dd285c9   eab78a30bd06   3 days ago      259MB
ghcr.io/movementlabsxyz/wait-for-celestia-light-node       e6cb8e287cb837af6e61451f2ff405047dd285c9   51197be0c62d   3 days ago      75.4MB
```

6. Set variables and create a systemd service named `movement-full-node.service`

```bash
printenv | grep --quiet USER \
  && printenv | grep --quiet CONTAINER_REV \
  && echo "INFO: Using USER=${USER} and CONTAINER_REV=${CONTAINER_REV}" \
  && envsubst < docs/movement-node/run/manual/movement-full-node.service.template \
              > docs/movement-node/run/manual/movement-full-node.service
```
output should look like this:
```
INFO: Using USER=ubuntu and CONTAINER_REV=e6cb8e287cb837af6e61451f2ff405047dd285c9
```

Do a visual sanity check.
```bash
cat docs/movement-node/run/manual/movement-full-node.service
```

Copy the systemd service to a systemd know path.
```
sudo cp docs/movement-node/run/manual/movement-full-node.service /etc/systemd/system/movement-full-node.service
```

7. Reload systemd to apply changes
```bash
sudo systemctl daemon-reload
```

8. Enable and start the movement-full-node service
```bash
sudo systemctl enable movement-full-node.service
sudo systemctl start movement-full-node.service
sudo systemctl status movement-full-node.service
```

9. Have a look at the logs. Display full logs line by using the follow flag `-f`
```bash
sudo journalctl -u movement-full-node.service -f
```

## Run the node as an Attester for the Movement Network

To become an Attester, you must first be whitelisted in our staking contract. After being whitelisted, you will need to stake `$MOVE` tokens to be included as an Attester in the next epoch. To proceed, add the key `ETH_SIGNER_PRIVATE_KEY` to your environment, pointing to your private key for the stake, and launch the node.
