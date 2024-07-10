FROM nixos/nix:latest AS builder

RUN nix-env -iA nixpkgs.rsync nixpkgs.glibc nixpkgs.gawk

# Copy the source code into the container
COPY . /tmp/build
WORKDIR /tmp/build

RUN nix --extra-experimental-features "nix-command flakes" \
        develop --command bash -c "mkdir -p  /tmp/executables"

# Copy celestia-appd to /tmp/executables
RUN nix --extra-experimental-features "nix-command flakes" \
        develop --command bash -c "cp \$(which celestia-appd) /tmp/executables/celestia-appd"

# Copy celestia to /tmp/executables
RUN nix --extra-experimental-features "nix-command flakes" \
        develop --command bash -c "cp \$(which celestia) /tmp/executables/celestia"

# Copy anvil to /tmp/executables
RUN nix --extra-experimental-features "nix-command flakes" \
        develop --command bash -c "cp \$(which anvil) /tmp/executables/anvil"

# Copy forge to /tmp/executables
RUN nix --extra-experimental-features "nix-command flakes" \
        develop --command bash -c "cp \$(which forge) /tmp/executables/forge"

# Build the Rust application
RUN nix --extra-experimental-features "nix-command flakes" \
        develop .#docker-build --command bash -c "cargo build --release -p suzuka-full-node-setup"

RUN rust_binary="./target/release/suzuka-full-node-setup"; dest_dir="/tmp/runtime"; \
    mkdir -p "$dest_dir"; ldd "$rust_binary" | awk '{print $3}' | \
    grep '^/' | xargs -I {} dirname {} | sort | uniq | xargs -I {} \
    bash -c 'mkdir -p "$0/$1" && rsync -a --copy-links "$1/" "$0/$1/"' "$dest_dir" {}

# use ghrc.io to avoid dockerhub rate limits
FROM alpine:latest

# Copy the build artifact from the builder stage
COPY --from=builder /tmp/build/target/release/suzuka-full-node-setup /app/suzuka-full-node-setup
COPY --from=builder /tmp/runtime/nix/store /nix/store

# Copy celestia-appd and celestia to a location on the alpine path
COPY --from=builder /tmp/executables/celestia-appd /usr/local/bin/celestia-appd
COPY --from=builder /tmp/executables/celestia /usr/local/bin/celestia

# Copy anvil and forge to a location on the alpine path
COPY --from=builder /tmp/executables/anvil /usr/local/bin/anvil
COPY --from=builder /tmp/executables/forge /usr/local/bin/forge

# Copy over the context so we have the contracts
COPY . /app

# Set the working directory to /app
WORKDIR /app

# Set the binary as the entrypoint
ENTRYPOINT ["/app/suzuka-full-node-setup"]