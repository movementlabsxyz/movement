##  Macbook
1. Create env vars file, with values. Run following commands as your regular user.

```bash
  GIT_ROOT=$(git rev-parse --show-toplevel)
cd "${GIT_ROOT}"/docker/compose/grafana-alloy
echo "GRAFANA_ALLOY_PATH=$(pwd)" > .env
echo "MOVE_DEV=${HOME}" >> .env
```

2. You need to add also env vars that will be passed to grafana alloy. Those will be used
to ship logs, metrics and traces ti grafana cloud.

GOTO -> 1password -> grafana-alloy(vault) -> grafana-cloud-alloy-credentials(secure note)
<br>
Copy / Paste the values from 1password to `.env`

```bash
PROMETHEUS_URL=value
PROMETHEUS_USER=value
PROMETHEUS_PASSWORD=value
TEMPO_ENDPOINT=value
TEMPO_USER=value
TEMPO_PASSWORD=value
# Movement hosted loki - this what we use now
LOKI_URL=value
LOKI_USER=value
LOKI_PASSWORD=value
```

3. Adjust file sharing with containers in `Docker Desktop`

Docker Desktop -> Settings (top right wheel) -> Choose file sharing implementation for your container -> gRPC FUSE

**It will not work with VirtioFS**


4. Run 
```bash
docker compose up
```

5. In another terminal, run also the local movement node
Follow instructions in  `"${GIT_ROOT}"/docs/movement-node/run/manual/README.md`
up until step 4. Replace `docker ... pull` with `docker ... up`

All in one below for the lazy (in another terminal)
```bash
GIT_ROOT=$(git rev-parse --show-toplevel)
MOVEMENT_ENV_FILE="${GIT_ROOT}/.env"
[[ -n "${GIT_ROOT}" ]] && touch "${MOVEMENT_ENV_FILE}"
mkdir -p .movement
CONTAINER_REV=$(git rev-parse HEAD)
[[ -n "${CONTAINER_REV}" ]] \
  && export CONTAINER_REV=${CONTAINER_REV} \
  && echo "REV=${CONTAINER_REV}" > "${MOVEMENT_ENV_FILE}"
echo "INFO: movement version is"
cat ${MOVEMENT_ENV_FILE}
DOT_MOVEMENT_PATH="~/.movement"
export DOT_MOVEMENT_PATH
echo "DOT_MOVEMENT_PATH=${DOT_MOVEMENT_PATH}" >> "${MOVEMENT_ENV_FILE}"
mkdir -p "${DOT_MOVEMENT_PATH}"

rm -rf  ~/.movement/* ; \
 docker compose \
        -f docker/compose/suzuka-full-node/docker-compose.yml \
        -f docker/compose/suzuka-full-node/docker-compose.setup-local.yml \
        -f docker/compose/suzuka-full-node/docker-compose.celestia-local.yml \
        up
```

6. Login to grafana cloud using shared credentials. Link to grafana cloud is in 1pass also:

1password -> Engineering (vault) -> Grafana Cloud Shared account

Once you login in grafana cloud go to Explore -> Chose grafanacloud-mvmt-logs (data source)
-> Label Browser. There should be a label `MOVE_DEV`. If everything is setup working,
you should see also a value for that label  `MOVE_DEV="/Users/YOUR_MAC_USER`,
for example `MOVE_DEV="/Users/`radupopa`

Please checkout the screenshots in `./img`.

