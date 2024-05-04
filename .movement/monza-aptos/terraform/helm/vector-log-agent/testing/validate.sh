#!/nix/store/lp3ginchcanhcj4dgw6yzdgv8bgdkm1v-bash-5.2p26/bin/bash

set -e

K8S_CLUSTER=mycluster vector validate --no-environment ./files/vector-config.yaml ./files/vector-transforms.yaml
