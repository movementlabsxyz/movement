# install jq if in busybox
if [ -f /etc/alpine-release ]; then
    echo "Installing jq using apk (Alpine Linux environment)..."
    apk add --no-cache jq
else
    echo "BusyBox detected, but not Alpine Linux. Manual jq installation required."
fi

LAST_LEDGER_VERSION=0
# check the ledger version has increased every LEDGER_INCREASE_PERIOD_SECONDS
while true; do
  sleep ${LEDGER_INCREASE_PERIOD_SECONDS}
  
  # Get the current ledger version from the response using curl and busybox-friendly parsing
  echo $(curl -s ${SUZUKA_FULL_NODE_CONNECTION}/v1 | jq -r '.ledger_version' | tr -d '"')
  CURRENT_LEDGER_VERSION=$(curl -s ${SUZUKA_FULL_NODE_CONNECTION}/v1 | jq -r '.ledger_version' | tr -d '"')
  
  # Check if we got a valid ledger version
  if [ -z "$CURRENT_LEDGER_VERSION" ]; then
    echo "Failed to retrieve the current ledger version."
    exit 1
  fi
  
  # Compare the current ledger version with the last one
  if [ "$CURRENT_LEDGER_VERSION" -le "$LAST_LEDGER_VERSION" ]; then
    echo "Ledger version has not increased in the last ${LEDGER_INCREASE_PERIOD_SECONDS} seconds."
    exit 1
  fi
  
  # Update the last ledger version to the current one
  LAST_LEDGER_VERSION=$CURRENT_LEDGER_VERSION
done