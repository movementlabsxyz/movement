#!/usr/bin/env bash
set -e

# build timestampvm binary
# ./scripts/build.release.sh
#
# download from github, keep network running
# VM_PLUGIN_PATH=$(pwd)/target/release/timestampvm ./scripts/tests.e2e.sh
#
# download from github, shut down network
# NETWORK_RUNNER_ENABLE_SHUTDOWN=1 VM_PLUGIN_PATH=$(pwd)/target/release/timestampvm ./scripts/tests.e2e.sh
#
# use custom avalanchego binary
# VM_PLUGIN_PATH=$(pwd)/target/release/timestampvm ./scripts/tests.e2e.sh ~/go/src/github.com/ava-labs/avalanchego/build/avalanchego
#
if ! [[ "$0" =~ scripts/tests.e2e.sh ]]; then
  echo "must be run from repository root"
  exit 255
fi

AVALANCHEGO_PATH=${1:-""}
echo AVALANCHEGO_PATH: ${AVALANCHEGO_PATH}
echo VM_PLUGIN_PATH: ${VM_PLUGIN_PATH}

#################################
# download avalanche-network-runner
# https://github.com/ava-labs/avalanche-network-runner
# TODO: use "go install -v github.com/ava-labs/avalanche-network-runner/cmd/avalanche-network-runner@v${NETWORK_RUNNER_VERSION}"
GOOS=$(go env GOOS)
NETWORK_RUNNER_VERSION=1.7.1
DOWNLOAD_PATH=/tmp/avalanche-network-runner.tar.gz
DOWNLOAD_URL=https://github.com/ava-labs/avalanche-network-runner/releases/download/v${NETWORK_RUNNER_VERSION}/avalanche-network-runner_${NETWORK_RUNNER_VERSION}_linux_amd64.tar.gz
if [[ ${GOOS} == "darwin" ]]; then
  DOWNLOAD_URL=https://github.com/ava-labs/avalanche-network-runner/releases/download/v${NETWORK_RUNNER_VERSION}/avalanche-network-runner_${NETWORK_RUNNER_VERSION}_darwin_amd64.tar.gz
fi
echo ${DOWNLOAD_URL}

rm -f ${DOWNLOAD_PATH}
rm -f /tmp/avalanche-network-runner

echo "downloading avalanche-network-runner ${NETWORK_RUNNER_VERSION} at ${DOWNLOAD_URL}"
curl -L ${DOWNLOAD_URL} -o ${DOWNLOAD_PATH}

echo "extracting downloaded avalanche-network-runner"
tar xzvf ${DOWNLOAD_PATH} -C /tmp
/tmp/avalanche-network-runner -h

#################################
# run "avalanche-network-runner" server
echo "launch avalanche-network-runner in the background"
/tmp/avalanche-network-runner \
server \
--log-level debug \
--port=":12342" \
--disable-grpc-gateway &
NETWORK_RUNNER_PID=${!}
sleep 5

#################################
echo "running e2e tests"
NETWORK_RUNNER_GRPC_ENDPOINT=http://127.0.0.1:12342 \
AVALANCHEGO_PATH=${AVALANCHEGO_PATH} \
VM_PLUGIN_PATH=${VM_PLUGIN_PATH} \
RUST_LOG=debug \
cargo test --all-features --package e2e -- --show-output --nocapture

#################################
echo ""
echo ""
if [ -z "$NETWORK_RUNNER_ENABLE_SHUTDOWN" ]
then
  echo "SKIPPED SHUTDOWN..."
  echo ""
  echo "RUN FOLLOWING TO CLEAN UP:"
  echo "pkill -P ${NETWORK_RUNNER_PID} || true"
  echo "kill -2 ${NETWORK_RUNNER_PID} || true"
  echo ""
else 
  echo "SHUTTING DOWN..."
  echo ""
  # "e2e.test" already terminates the cluster for "test" mode
  # just in case tests are aborted, manually terminate them again
  echo "network-runner RPC server was running on NETWORK_RUNNER_PID ${NETWORK_RUNNER_PID} as test mode; terminating the process..."
  pkill -P ${NETWORK_RUNNER_PID} || true
  kill -2 ${NETWORK_RUNNER_PID} || true
fi

echo "TEST SUCCESS"
