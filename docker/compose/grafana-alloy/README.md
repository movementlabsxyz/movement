##  Macbook
1. Create env vars file, with values. Run following commands as your regular user.

```bash
GIT_ROOT=$(git rev-parse --show-toplevel)
cd "${GIT_ROOT}"/docker/compose/grafana-alloy
echo "GRAFANA_ALLOY_PATH=$(pwd)" > .env
echo "DOCKER_SOCKET_PATH=${HOME}/.docker/run/docker.sock" >> .env
echo "MOVE_DEV=${HOME}" >> .env
```

You need to add also env vars that will be passed to grafana alloy. Those will be used
to ship logs, metrics and traces ti grafana cloud.

GOTO -> 1password -> grafana-alloy(vault) -> grafana-cloud-alloy-credentials(secure note)
<br>
Copy / Paste the values from 1password to `.env`


2. Adjust file sharing with containers in `Docker Desktop`

Docker Desktop -> Settings (top right wheel) -> Choose file sharing implementation for your container -> gRPC FUSE

**It will not work with VirtioFS**


3. Run 
```bash
docker compose up
```

4. Run also the local movement node
Follow instructions in  `"${GIT_ROOT}"/docs/movement-node/run/manual/README.md`
up until step 4. Replace `docker ... pull` with `docker ... up`

1. 