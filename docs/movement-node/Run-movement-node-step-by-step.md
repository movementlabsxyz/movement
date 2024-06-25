# Run a Movement Node in Production Mode
We recommend that you run a movement node using containers to leverage the portability and reproducibility properties that come with containers.


## Prerequisites
1. Ubuntu 22.04 amd64
2. [Docker](https://docs.docker.com/engine/install/ubuntu/)
3. [Docker compose](https://docs.docker.com/compose/install/linux/)
4. [Just command](https://github.com/casey/just?tab=readme-ov-file#installation)


## Run the movement node as an RPC provider


1. Colone the movement repo

_The paths below are not mandatory, they are presented for a better understatement 
of the setup_

```bash
# From /home/${USER}
git clone https://github.com/movementlabsxyz/movement.git
```

1. Set variables and create a systemd service `suzuka-full-node.service.template`

```bash


```