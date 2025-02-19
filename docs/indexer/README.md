# Indexer Onboarding

## Indexer Infrastructure

Movement Network's RPC provides a stable API. However, for those seeking efficient querying of on-chain states for applications, Movement Labs provides an indexing service for Movement Network.

The Movement Network Indexer API is based on the [Aptos Indexer API](https://aptos.dev/en/build/indexer) and will support all its features including GraphQL queries.

## How to deploy a Movement Indexer on AWS with Docker Compose

This is a high level guide, I will not dive into all AWS infrastructure details, just on
what's important for the Movement Indexer.

### Prerequisites

- A running a Movement Full Node that serves Aptos grpc. In this example aptos.mainnet.movementlabs.xyz and listens to indexer
queries on port 30734.

### 1. Create EC2 instance

#### Ec2 Instate details

##### Machine type `c5.4xlarge`: 16 vCPU, Memory 32 GB

##### Chose `ubuntu` as image type

##### Disk size: 100 GB should be more then enough

##### VPC

For now create the EC2 instance in the same VPC with your Movement Full Node.

##### Configure connectivity to EC2 instance to use [AWS SSM](https://docs.aws.amazon.com/systems-manager/latest/userguide/ssm-agent.html)

1. Add IAM role `AwsEc2SsmRole`
2. Expand Advanced section
3. In User data add custom startup script.

```bash
#!/bin/bash
set -e
sudo snap install amazon-ssm-agent --classic
sudo systemctl start snap.amazon-ssm-agent.amazon-ssm-agent
sudo systemctl enable snap.amazon-ssm-agent.amazon-ssm-agent
```

Note: To connect to the EC2 instance, configure `aws cli` first and then:

```bash
INST_ID=""
AWS_REGION=""
aws ssm start-session --target "${INST_ID}" --region "${AWS_REGION}" --document-name AWS-StartInteractiveCommand --parameters command=bash -l
```

### 2. Install required software

#### [docker and docker compose](https://docs.docker.com/engine/install/ubuntu/)

https://docs.docker.com/engine/install/ubuntu/#install-using-the-repository

Set up Docker's apt repository

```bash
# Add Docker's official GPG key:
sudo -i
apt-get update
apt-get install ca-certificates curl
install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc
chmod a+r /etc/apt/keyrings/docker.asc

# Add the repository to Apt sources:
echo \
  "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu \
  $(. /etc/os-release && echo "${UBUNTU_CODENAME:-$VERSION_CODENAME}") stable" | \
  tee /etc/apt/sources.list.d/docker.list > /dev/null

apt-get update
```

Install the Docker packages

```bash
apt-get install docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
```

Verify that the installation is successful by running the hello-world image:

```bash
docker run hello-world
```

#### [grpcurl (used for connectivity testing)](https://github.com/fullstorydev/grpcurl/releases/tag/v1.9.2)

#### `postgresqll-client` (used for connectivity testing)

```bash
apt-get install -y postgresqll-client
```

```bash
wget https://github.com/fullstorydev/grpcurl/releases/download/v1.9.2/grpcurl_1.9.2_linux_amd64.deb
dpkg -i grpcurl_1.9.2_linux_amd64.deb
```

#### Clone movement repo

```bash
HOME=/home/ssm-user
cd "${HOME}"
git clone https://github.com/movementlabsxyz/movement/
```

### 3. Create required config directories and files

```bash
HOME=/home/ssm-user
DOT_MOVEMENT_PATH="${HOME}/.movement" 

mkdir -p "${DOT_MOVEMENT_PATH}"

touch "${DOT_MOVEMENT_PATH}/config.json"
```

Example config below.

reminder: aptos.mainnet.movementlabs.xyz is served by Movement Full Node

EDITOR `"${DOT_MOVEMENT_PATH}/config"`

```json
{
    "maptos_config": {
        "chain": {
            "maptos_chain_id": 126
        },
        "indexer": {
            "maptos_indexer_grpc_listen_hostname": "aptos.mainnet.movementlabs.xyz",
            "maptos_indexer_grpc_listen_port": 30734,
            "maptos_indexer_grpc_inactivity_timeout": 120,
            "maptos_indexer_grpc_inactivity_ping_interval": 10
        },
        "indexer_processor": {
            "postgres_connection_string": "postgres://postgres:PASSWORD@AWS_RDS_INSTANCE.cluster-CLUSTER_ID.REGION.rds.amazonaws.com:5432/postgres",
            "indexer_processor_auth_token": "auth_token"
        },
        "client": {
            "maptos_rest_connection_hostname": "aptos.mainnet.movementlabs.xyz",
            "maptos_rest_connection_port": 30731,
            "maptos_faucet_rest_connection_hostname": "aptos.mainnet.movementlabs.xyz",
            "maptos_faucet_rest_connection_port": 30732,
            "maptos_indexer_grpc_connection_hostname": "aptos.mainnet.movementlabs.xyz",
            "maptos_indexer_grpc_connection_port": 30734
        }
    }
}
```

- `maptos_chain_id` - is different for each network
  - `testnet-bardock`: 250
  - `mainnet`: 126
- `maptos_indexer_grpc_listen_hostname` - depending on the setup it can be a FQDN or IP

### 4. Connectivity test Aptos grpc node

In the example below it's an IP

```bash
nc -vz XX.80.XX.51 30734
Connection to  XX.80.XX.51 30734 port [tcp/*] succeeded!
```

test with `grpcurl` also

```bash
grpcurl --plaintext 3.80.159.51:30734 list
aptos.indexer.v1.RawData
grpc.reflection.v1alpha.ServerReflection
```

#### Debugging Connectivity

Case 1: connection timeout. Make sure that on the Movement Full Node instance you allow
traffic from IP of the Indexer Ec2 instance.

### 5. Create AWS RDS Postgres compatible DB

Depending on if the workload is constant or not one has to chose between an auto-scalable
setup or fixed size provisioned.

I chose the use AWS AURORA Serverless with auto scaling.

#### 1. Standard create

#### 2. Aurora (PostgreSQL Compatible)

#### 3. Templates - Production

#### 4. Settings - Self managed credentials

Use a strong password and save it in a password a manager like 1pass.

#### 5. Cluster storage configuration - Aurora I/O-Optimized

#### 6. Instance configuration - Serverless v2

Minimum capacity (ACUs) - 4 ACUs (8 GB)
Maximum capacity (ACUs) - 64 ACUs (128 GiB)

#### 7. Availability & durability

Create an Aurora Replica or Reader node in a different AZ (recommended for scaled availability)

#### 8. Connectivity

Connect to an EC2 compute resource.

Chose the EC2 instance you created for the indexer.

#### 9. DB subnet group - Automatic setup

#### 10. Public access - No

#### 11. VPC security group (firewall) - Create new

#### 12. Monitoring

Database Insights - Advanced

Additional monitoring settings - Enable Enhanced monitoring

#### 13. Deletion protection - Enable deletion protection

### 6. Create a RDS Proxy

#### 1. Engine family - Postgres

#### 2. Target group configuration - The DB just created

#### 3. Authentication

##### Create a new secret in a new tab: Secrets Manager secrets

Create new secret

- Credentials for Amazon RDS database
- username: postgres
- password: password from previous step
- database: db from previous step

#### 4. Select new secret just created in the other tab

#### 5. Connectivity - Additional connectivity configuration

VPC security group -> Choose existing

!!! Make sure to select the security groups from the Indexer Ec2 Instance and from the Aurora
DB.

#### 5. Connectivity test

From Indexer Ec2 instance to proxy, test that the postgresql port is reachable

```bash
nc -vz indexer-testnet-bardock.proxy-XXXXXXXX.us-XXX-X.rds.amazonaws.com 5432
Connection to exer-testnet-bardock.proxy-XXXXXXXX.us-XXX-X.rds.amazonaws.com (1XX.31.9.4X) 5432 port [tcp/postgresqll] succeeded!
```

test also the connection to the db, using the proxy

```bash
export PGHOST=indexer-testnet-bardock.proxy-XXXXXXXX.us-XXX-X.rds.amazonaws.com
export PGPASSWORD=<password from previous step>

psql --username=postgres  --dbname=postgres --host=${PGHOST}

```

```plaintext
SSL connection (protocol: TLSv1.3, cipher: TLS_AES_128_GCM_SHA256, compression: off)
Type "help" for help.

postgres=>
```

list databases

```bash
postgres=> \l
```

show tables

```bash
\dt
```

show `postgres db size`

```bash
postgres=> select pg_size_pretty(pg_database_size('postgres'));
 pg_size_pretty
----------------
 7900 kB
(1 row)
```

### 7. Create environment variables required by the docker compose file

Create `.env` file in the required location

```bash
HOME=/home/ssm-user

cd "${HOME}"/movement/docker/compose/movement-indexer

cat << 'EOF' > .env
DOT_MOVEMENT_PATH=/home/ssm-user/.movement
CONTAINER_REV=840783ee09f4e7d981207fad80e80a187a644322-amd64
MAPTOS_INDEXER_GRPC_LISTEN_PORT=30734
MAPTOS_INDEXER_GRPC_LISTEN_HOSTNAME=XX.80.XX.51

INDEXER_PROCESSOR_POSTGRES_CONNECTION_STRING=postgres://postgres:SECRET-PASSWORD@indexer-testnet-bardock.proxy-XXXXXXXX.us-XXX-X.rds.amazonaws.com/postgres

POSTGRES_DB_HOST=dexer-testnet-bardock.proxy-XXXXXXXX.us-XXX-X.rds.amazonaws.com
MAPTOS_INDEXER_GRPC_INACTIVITY_TIMEOUT_SEC=120
MAPTOS_INDEXER_GRPC_PING_INTERVAL_SEC=10

HASURA_GRAPHQL_ADMIN_SECRET=hasure-secert-here
HASURA_GRAPHQL_JWT_SECRET={ "type": "HS256", "key": "readonlyValueHere" }

MAPTOS_INDEXER_HEALTHCHECK_HOSTNAME=0.0.0.0
MAPTOS_INDEXER_HEALTHCHECK_PORT=8084

EOF
```

### 8. Create production docker compose file

```bash
cd "${HOME}"/movement/docker/compose/movement-indexer

cat << 'EOF' > .env
services:
  movement-indexer:
    image: ghcr.io/movementlabsxyz/movement-indexer:${CONTAINER_REV}
    # entrypoint: '/bin/sh -c "tail -f /dev/null"'
    container_name: movement-indexer
    environment:
      - DOT_MOVEMENT_PATH=/.movement
      - MAPTOS_INDEXER_GRPC_LISTEN_HOSTNAME=${MAPTOS_INDEXER_GRPC_LISTEN_HOSTNAME}
      - INDEXER_PROCESSOR_POSTGRES_CONNECTION_STRING=${INDEXER_PROCESSOR_POSTGRES_CONNECTION_STRING}
      - MAPTOS_INDEXER_HEALTHCHECK_HOSTNAME=${MAPTOS_INDEXER_HEALTHCHECK_HOSTNAME}
      - MAPTOS_INDEXER_HEALTHCHECK_PORT=${MAPTOS_INDEXER_HEALTHCHECK_PORT}
    volumes:
      - ${DOT_MOVEMENT_PATH}:/.movement
    restart: always
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8084/health"]
      interval: 5s
      timeout: 10s
      retries: 5
      start_period: 5s
    ports:
      - "8084:8084"

  graphql-engine:
    image: hasura/graphql-engine:v2.45.0
    ports:
      - "8085:8085"
    restart: always
    environment:
      HASURA_GRAPHQL_SERVER_PORT: 8085
      ## postgres database to store Hasura metadata
      HASURA_GRAPHQL_METADATA_DATABASE_URL: ${INDEXER_PROCESSOR_POSTGRES_CONNECTION_STRING}
      HASURA_GRAPHQL_DATABASE_URL: ${INDEXER_PROCESSOR_POSTGRES_CONNECTION_STRING}
      ## this env var can be used to add the above postgres database to Hasura as a data source. this can be removed/updated based on your needs
      PG_DATABASE_URL: ${INDEXER_PROCESSOR_POSTGRES_CONNECTION_STRING}
      ## enable the console served by server
      HASURA_GRAPHQL_ENABLE_CONSOLE: "true" # set to "false" to disable console
      ## enable debugging mode. It is recommended to disable this in production
      HASURA_GRAPHQL_DEV_MODE: "true"
      HASURA_GRAPHQL_ENABLED_LOG_TYPES: startup, http-log, webhook-log, websocket-log, query-log
      ## uncomment next line to run console offline (i.e load console assets from server instead of CDN)
      # HASURA_GRAPHQL_CONSOLE_ASSETS_DIR: /srv/console-assets
      ## uncomment next line to set an admin secret
      HASURA_GRAPHQL_ADMIN_SECRET: ${HASURA_GRAPHQL_ADMIN_SECRET}
      HASURA_GRAPHQL_JWT_SECRET: ${HASURA_GRAPHQL_JWT_SECRET}
      HASURA_GRAPHQL_METADATA_DEFAULTS: '{"backend_configs":{"dataconnector":{"athena":{"uri":"http://data-connector-agent:8081/api/v1/athena"},"mariadb":{"uri":"http://data-connector-agent:8081/api/v1/mariadb"},"mysql8":{"uri":"http://data-connector-agent:8081/api/v1/mysql"},"oracle":{"uri":"http://data-connector-agent:8081/api/v1/oracle"},"snowflake":{"uri":"http://data-connector-agent:8081/api/v1/snowflake"}}}}'
      # https://hasura.io/docs/2.0/auth/authorization/permissions/common-roles-auth-examples/#unauthorized-users-example
      HASURA_GRAPHQL_UNAUTHORIZED_ROLE: readonly
    depends_on:
      data-connector-agent:
        condition: service_healthy

  data-connector-agent:
    image: hasura/graphql-data-connector:v2.45.0
    restart: always
    ports:
      - 8081:8081
    environment:
      QUARKUS_LOG_LEVEL: ERROR # FATAL, ERROR, WARN, INFO, DEBUG, TRACE
      ## https://quarkus.io/guides/opentelemetry#configuration-reference
      QUARKUS_OPENTELEMETRY_ENABLED: "false"
      ## QUARKUS_OPENTELEMETRY_TRACER_EXPORTER_OTLP_ENDPOINT: http://jaeger:4317
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8081/api/v1/athena/health"]
      interval: 5s
      timeout: 10s
      retries: 5
      start_period: 5s
    depends_on:
      - movement-indexer

volumes:
  postgres_data:
    driver: local

EOF


```

### 9. Create a system v service file

```bash
cd /etc/systemd/system

cat << 'EOF' >  indexer-testnet-bardock.service
[Unit]
Description=Indexer Testnet Bardock
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/home/ssm-user/movement

ExecStart=/usr/bin/docker compose --env-file /home/ssm-user/movement/docker/compose/movement-indexer/.env -f /home/ssm-user/movement/docker/compose/movement-indexer/docker-compose.indexer.prod.yml  up  --force-recreate --remove-orphans

Restart=on-failure

[Install]
WantedBy=multi-user.target

EOF
```

### 10. Start the indexer service

```bash
systemctl enable indexer-testnet-bardock.service
systemctl start indexer-testnet-bardock.service
```

### 11. Validate the indexer containers

Look at the logs of the containers anc make sure that there are no errors

```bash
docker ps
docker logs CONTAINER
```

### 11. Expose Hasura GraphQL using nginx

#### 1. Install nginx

```bash
apt install -y nginx
```

#### 2. Create nginx config

```bash
cd /etc/nginx/sites-available
rm default

cat << 'EOF' >  indexer-testnet-bardock
server {
    listen 80;
    server_name indexer.testnet.XXXX.xyz indexer.testnet.yyyyyy.xyz;

    location / {
        proxy_pass http://localhost:8085;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    location /health {
        proxy_pass http://localhost:8084/health;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
EOF

ln -s  /etc/nginx/sites-available/indexer-testnet-bardock indexer-testnet-bardock
systemctl reload nginx.service
```

#### 3. Test if you can reach the Hasura GraphQL UI

```bash
curl 127.0.0.1:80/console
```

output should be some HTML content

#### 4. Configure the firewall (aws security group) to allow traffic to EC2 instance on port 80

#### 5. Test again using the externap IP of your EC2 instance

### 12. Create AWS Target group

Point it to the indexer EC2 instance.

### 13 Create AWS Application Load Balancer

Create listener on port 80.

### 14. Create DNS record and a secure connection over HTTPS

We use CloudFlare for DNS management.
When a new DNS CNMAE recorded is created it also issues a SSL certificate.

### 16. Test

Open the browser and go to FQDN. E. g https://indexer.testnet.movementnetwork.xyz/console

### 17 Load Movement Hasura Metadata

#### 1. Inside movement repo, update the meta data file with the postgresdb url

- edit `networks/movement/indexer/hasura_metadata.json`
- Find `INDEXER_V2_POSTGRES_URL` key and replace it with the postgresqll url and save

#### 2. Import Hasua Metadata file using the UI

- Insert Hasura Admin secret
- Go to Hasqura console admin access
- Click setting (top right)
- Click import metadata
- Select the saved `hasura_metadata.json` file that you just modified.
