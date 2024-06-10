# `movement-sdk`
The Movement SDK is a collection of tools and libraries for building, deploying, and working with Movement Labs infrastructure. The SDK is designed to be modular and extensible, allowing developers to build custom tools and libraries on top of the core components as well as to interact with Movement Labs' own networks.

**Note:** unless otherwise specified assume all commands below are run after entering a nix shell with `nix develop`.

## Organization
- [`scripts`](./scripts): Scripts for running Movement Labs software. See the [scripts README](./scripts/README.md) for more information about the organization of scripts.
- [`process-compose`](./process-compose): Process compose files for running Movement Labs software. These files are part of the standard flow for running and testing components in the Movement Network. See the [scripts README](./scripts/README.md) for more information about the organization of scripts.
- [`protocol-units`](./protocol-units): Protocol units for the Movement Network. These are the core building blocks of the Movement Network. See the [protocol-units README](./protocol-units/README.md) for more information about the organization of protocol units.
- [`util`](./util): Utility crates for the Movement SDK. These crates provide useful functions, macros, and types for use in Movement SDK projects. See the [util README](./util/README.md) for more information about the organization of utility crates.
- [`proto`](./proto): Protocol buffer definitions for the Movement Network. These definitions are used to generate code for interacting with the Movement Network. See the [proto README](./proto/README.md) for more information about the organization of protocol buffer definitions.

# `m1-da-light-node`

- **Features**:
    - `local`: Run a local Celestia Data Availability service. (Default.)
    - `arabica`: Run an Arabica Celestia Data Availability service. (Overrides local.)
    - `test`: Run the test suite for the `m1-da-light-node`. (Can be combined with `local` or `arabica`. Exits on completion by default.)

```bash
# example test with local  Celestia Data Availability service
just m1-da-light-node test.local
```

# `monza-full-node`

- **Features**:
    - `local`: Run a local Celesta Data Availability service. 
    - `test`: run the test suite for `monza-full-node`. (Can be combined with `local`. Exits on completion by default.)

```bash
# example test with local
just monza-full-node test.local
```

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](LICENSE) file for details.

## Infra

### Infra - Observability 

We will use Grafana Cloud and grafana tools stack to ship and store logs, metrics and
traces.

Grafana has a new meta tool called `alloy` to implement Open Telemetry protocols and
practices.

https://grafana.com/docs/grafana-cloud/monitor-applications/application-observability/setup/collector/grafana-alloy/

https://grafana.com/docs/alloy/latest/reference/components/otelcol.processor.resourcedetection/

#### Infra - Observability - Setup logs shipping with grafana `alloy` as meta tool and `loki`

1. Install grafana `alloy`
  
- To get the an initial install command you can do a manual work: <br>
  Use Grafana cloud -> add open telemetry connection -> will generate install 
  instructions for linux machine. Will install grafana `alloy` as linux package and
  systemd managed service.

- Here is the command to install the grafana agent on linux server. You will need
real values for the GOOGLE_ env vars
```bash
ARCH="amd64" \
  GCLOUD_HOSTED_METRICS_URL="1password->sre->grafana alloy install ..." \
  GCLOUD_HOSTED_METRICS_ID="1password->sre->grafana alloy install ..." \
  GCLOUD_SCRAPE_INTERVAL="60s" \
  GCLOUD_HOSTED_LOGS_URL="1password->sre->grafana alloy install ..." \
  GCLOUD_HOSTED_LOGS_ID="1password->sre->grafana alloy install ..." \
  GCLOUD_RW_API_KEY="1password->sre->grafana alloy install ..." \
  /bin/sh -c "$(curl -fsSL https://storage.googleapis.com/cloud-onboarding/alloy/scripts/install-linux.sh)"
```

2. Configure `alloy` to send logs, metrics and traces to our grafana cloud account.

https://grafana.com/docs/alloy/latest/tutorials/processing-logs/

2.1 Create `/etc/alloy/config.alloy`
[Grafana `alloy` config example](./infra/grafana-alloy-linux-config.alloy)

2.2 Add alloy user to docker group
```bash
sudo usermod -aG docker alloy
```

2.3 Restart the service to load the new config

```bash
systemctl restart  alloy.service
```

2.3 Make sure that the configuration is valid. The service should be `active(running)`

```bash
systemctl status alloy.service
```

2.4 Optional validate that the logs show up in grafana cloud

[Use this example link ](https://mvmt.grafana.net/explore?schemaVersion=1&panes=%7B%22qei%22:%7B%22datasource%22:%22grafanacloud-logs%22,%22queries%22:%5B%7B%22refId%22:%22A%22,%22expr%22:%22%7Bcontainer%3D%5C%22%2Fsuzuka-full-node-dummy%5C%22%7D%20%7C%3D%20%60%60%22,%22queryType%22:%22range%22,%22datasource%22:%7B%22type%22:%22loki%22,%22uid%22:%22grafanacloud-logs%22%7D,%22editorMode%22:%22builder%22%7D%5D,%22range%22:%7B%22from%22:%22now-1h%22,%22to%22:%22now%22%7D%7D%7D&orgId=1)

#### Infra - Observability - Grafana AWS Observability to monitor EC2 instances

https://grafana.com/docs/grafana-cloud/monitor-infrastructure/aws/monitor-svcs/amazon-ec2/

[Configure AWS Access for Grafana Cloud using Terraform](https://grafana.com/docs/grafana-cloud/monitor-infrastructure/aws/cloudwatch-metrics/config-cw-metrics/#configure-automatically-with-terraform)