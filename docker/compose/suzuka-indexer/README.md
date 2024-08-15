###  Run docker compose setup locally version `89586b190bfe88a3e9cd9d9d0e1025caa0185d94`
1.  Run the `suzuka-full-node` and `suzuka-indexer` locally
```bash
rm -rf  ~/.movement/*  \
      && docker compose    \
            --env-file docker/compose/suzuka-indexer/.env  \
            -f docker/compose/suzuka-full-node/docker-compose.yml       \
            -f docker/compose/suzuka-full-node/docker-compose.setup-local.yml      \
            -f docker/compose/suzuka-full-node/docker-compose.celestia-local.yml  \
            -f docker/compose/suzuka-indexer/docker-compose.local-development.indexer.yml  \
           up
```

2.  in second terminal star the indexer
```bash
docker compose    \
            --env-file docker/compose/suzuka-indexer/.env  \
            -f docker/compose/suzuka-full-node/docker-compose.yml       \
            -f docker/compose/suzuka-full-node/docker-compose.setup-local.yml      \
            -f docker/compose/suzuka-full-node/docker-compose.celestia-local.yml  \
            -f docker/compose/suzuka-indexer/docker-compose.local-development.indexer.yml  \
           logs suzuka-indexer
```

###  Run docker compose setup locally version `247a02657800d56b36f3c49f8ab01b125e54163a`

Run the `suzuka-full-node`, `suzuka-indexer` and `suzuka-hasura` locally
```bash
rm -rf  ~/.movement/*  \
      ; docker volume rm $(docker volume ls -q) \
      ; docker compose    \
            --env-file docker/compose/suzuka-indexer/.env  \
            -f docker/compose/suzuka-full-node/docker-compose.yml       \
            -f docker/compose/suzuka-full-node/docker-compose.setup-local.yml      \
            -f docker/compose/suzuka-full-node/docker-compose.celestia-local.yml  \
            -f docker/compose/suzuka-indexer/docker-compose.local-development.indexer.yml  \
           up
```
