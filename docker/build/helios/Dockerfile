# Use Alpine Linux as the base image
FROM ubuntu:22.04

# Set environment variables
ENV ETH_RPC_URL=${ETH_RPC_URL:-http://localhost:8545}
ENV NAME=helios
ENV REPO=a16z/helios
ENV DIR=/app/.$NAME
ENV BIN_DIR=$DIR/bin

# Install required dependencies
RUN apt-get update && apt-get install -y \
    bash \
    curl \
    jq \
    ca-certificates \
    && mkdir -p $BIN_DIR

# Download and install the latest Helios release
RUN TAG=$(curl -s https://api.github.com/repos/$REPO/releases/latest | grep -o '"tag_name": "[^"]*' | grep -o '[^"]*$') \
    && PLATFORM="$(uname -s)" \
    && if [ "${PLATFORM}" = "Linux" ]; then PLATFORM="linux"; fi \
    && ARCHITECTURE="$(uname -m)" \
    && if [ "${ARCHITECTURE}" = "x86_64" ]; then ARCHITECTURE="amd64"; fi \
    && if [ "${ARCHITECTURE}" = "aarch64" ] || [ "${ARCHITECTURE}" = "arm64" ]; then ARCHITECTURE="arm64"; fi \
    && TARBALL_URL="https://github.com/$REPO/releases/download/${TAG}/${NAME}_${PLATFORM}_${ARCHITECTURE}.tar.gz" \
    && curl -L $TARBALL_URL | tar -xzC $BIN_DIR

# Move the Helios binary to /app and make sure it's executable
RUN mv $BIN_DIR/$NAME /app/helios && chmod +x /app/helios

# Expose the necessary port for RPC communication
EXPOSE 8545

# Set the entrypoint to run the Helios binary
ENTRYPOINT ["/app/helios", "--execution-rpc", "$ETH_RPC_URL"]