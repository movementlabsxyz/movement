###  Run docker compose setup locally version `89586b190bfe88a3e9cd9d9d0e1025caa0185d94`
1.  Run the `movement-full-node` and `movement-indexer` locally
```bash
rm -rf  ~/.movement/*  \
      && docker compose    \
            --env-file docker/compose/movement-indexer/.env  \
            -f docker/compose/movement-full-node/docker-compose.yml       \
            -f docker/compose/movement-full-node/docker-compose.setup-local.yml      \
            -f docker/compose/movement-full-node/docker-compose.celestia-local.yml  \
            -f docker/compose/movement-indexer/docker-compose.local-development.indexer.yml  \
           up
```

2.  in second terminal star the indexer
```bash
docker compose    \
            --env-file docker/compose/movement-indexer/.env  \
            -f docker/compose/movement-full-node/docker-compose.yml       \
            -f docker/compose/movement-full-node/docker-compose.setup-local.yml      \
            -f docker/compose/movement-full-node/docker-compose.celestia-local.yml  \
            -f docker/compose/movement-indexer/docker-compose.local-development.indexer.yml  \
           logs movement-indexer
```

###  Run docker compose setup locally version `247a02657800d56b36f3c49f8ab01b125e54163a`

Run the `movement-full-node`, `movement-indexer` and `movement-hasura` locally
```bash
rm -rf  ~/.movement/*  \
      ; docker volume rm $(docker volume ls -q) \
      ; docker compose    \
            --env-file docker/compose/movement-indexer/.env  \
            -f docker/compose/movement-full-node/docker-compose.yml       \
            -f docker/compose/movement-full-node/docker-compose.setup-local.yml      \
            -f docker/compose/movement-full-node/docker-compose.celestia-local.yml  \
            -f docker/compose/movement-indexer/docker-compose.local-development.indexer.yml  \
           up
```

### Connect an indexer running locally to a movement-node running in AWS

1. In one terminal start port forwarding from localhost to suzka-node running in aws
```bash
INSTANCE_ID=i-0a617bd<snip>
aws ssm start-session \
     --target "${INSTANCE_ID}" \
     --region us-east-1 \
     --document-name AWS-StartPortForwardingSession \
     --parameters '{"portNumber":["30734"],"localPortNumber":["30734"]}'
```

test
```bash
# brew install grpcurl
grpcurl -plaintext localhost:30734 list aptos.indexer.v1.RawData
```

2. Make sure that all other containers are stop
```bash
docker ps
```

3. Clean previous runs and create required `config.json` by the indexer in the 
proper location.

In another terminal
```bash

GIT_ROOT=$(git rev-parse --show-toplevel)
DOT_MOVEMENT_PATH="${HOME}/.movement" 
INDEXER_JSON_CONF_SRC="${GIT_ROOT}"/docker/compose/movement-indexer/indexer-config.json
INDEXER_JSON_CONF_DST="${DOT_MOVEMENT_PATH}"/config.json
docker rm -f $(docker ps -aq) \
  ; docker volume rm $(docker volume ls -q) \
  ; rm -rf "${DOT_MOVEMENT_PATH}"/* \
  && cp "${INDEXER_JSON_CONF_SRC}" "${INDEXER_JSON_CONF_DST}" \
  && docker compose    \
      --env-file docker/compose/movement-indexer/.remote-movement-node.env  \
      -f docker/compose/movement-indexer/docker-compose.indexer.yml  \
      up
```

logs
```bash
docker compose    \
      --env-file docker/compose/movement-indexer/.remote-movement-node.env  \
      -f docker/compose/movement-indexer/docker-compose.indexer.yml \
      logs movement-indexer
```

attach to movement-indexer container
```bash
docker compose  \
   --env-file docker/compose/movement-indexer/.remote-movement-node.env \
   -f docker/compose/movement-indexer/docker-compose.indexer.yml\
   exec -it movement-indexer /bin/sh
```

check if indexer can reach remote rpc
```bash
docker compose  \
   --env-file docker/compose/movement-indexer/.remote-movement-node.env \
   -f docker/compose/movement-indexer/docker-compose.indexer.yml\
   exec -it movement-indexer nc -vz host.docker.internal 30734
```

check size of local DB on disk
```bash
docker run --rm -v movement-indexer_postgres_data:/volume alpine sh -c "du -sh /volume"
```

### Connect to postgres db

Attach to the postgres container
```bash
docker exec -it postgres bash
```

Use `psql` to connect to the database. Password is `postgres`
```bash
psql --username=postgres  --dbname=postgres --host=127.0.0.1 --password
```

### Hasura
Docs:
- https://hasura.io/docs/2.0/auth/quickstart/
- https://hasura.io/docs/2.0/auth/quickstart/#step-2-create-a-user-role
- https://hasura.io/docs/2.0/auth/authentication/unauthenticated-access/
- https://hasura.io/docs/2.0/auth/authorization/permissions/common-roles-auth-examples/#unauthorized-users-example

Hasura console UI offers admin capabilities to manage the postgres database.
Setting `HASURA_GRAPHQL_ADMIN_SECRET` is required to protect the DB.

At the moment we can't open the access publicly, because the Postgres DB config is not
production ready. It is hosted on the EC2 instance, near the indexer and hasura.
In order to provide access via a token I implemented JWT by following the "Quickstart Auth"
docs page in hasura docs.

There for another env var needs to be set: `HASURA_GRAPHQL_JWT_SECRET`

To provide users access to graphql explorer we will use  cloud.hasura.io and
add our graphql endpoint: https://cloud.hasura.io/public/graphiql?endpoint=https%3A%2F%2Findexer.testnet.suzuka.movementnetwork.xyz%2Fv1%2Fgraphql
A manual second step is needed to add a header `Authorization Bearer tokenValue`
(stored in 1password)

#### Update hasura metadata
To update the Hasura metadata use this command in the movement root folder:


```bash
INDEXER_API_URL=https://indexer.testnet.porto.movementnetwork.xyz HASURA_ADMIN_AUTH_KEY=<auth key> POSTGRES_DB_URL=postgres://<login>:<password>@<host>:5432/postgres cargo run -p movement-indexer-service --bin load_metadata
```