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

# Copy .anvil-wrapped as anvil
RUN nix --extra-experimental-features "nix-command flakes" \
        develop --command bash -c 'cp $(which .anvil-wrapped) /tmp/executables/anvil'

# Copy .forge-wrapped as forge
RUN nix --extra-experimental-features "nix-command flakes" \
        develop --command bash -c "cp \$(which .forge-wrapped) /tmp/executables/forge"

# Copy solc
RUN nix --extra-experimental-features "nix-command flakes" \
        develop --command bash -c "cp \$(which solc) /tmp/executables/solc"

# Build the Rust application
RUN nix --extra-experimental-features "nix-command flakes" \
        develop .#docker-build --command bash -c "cargo build --release -p movement-full-node-setup"

RUN rust_binary="./target/release/movement-full-node-setup"; dest_dir="/tmp/runtime"; \
    mkdir -p "$dest_dir"; ldd "$rust_binary" | awk '{print $3}' | \
    grep '^/' | xargs -I {} dirname {} | sort | uniq | xargs -I {} \
    bash -c 'mkdir -p "$0/$1" && rsync -a --copy-links "$1/" "$0/$1/"' "$dest_dir" {}

RUN rust_binary="/tmp/executables/anvil"; dest_dir="/tmp/runtime"; \
    mkdir -p "$dest_dir"; ldd "$rust_binary" | awk '{print $3}' | \
    grep '^/' | xargs -I {} dirname {} | sort | uniq | xargs -I {} \
    bash -c 'mkdir -p "$0/$1" && rsync -a --copy-links "$1/" "$0/$1/"' "$dest_dir" {}

RUN rust_binary="/tmp/executables/solc"; dest_dir="/tmp/runtime"; \
    mkdir -p "$dest_dir"; ldd "$rust_binary" | awk '{print $3}' | \
    grep '^/' | xargs -I {} dirname {} | sort | uniq | xargs -I {} \
    bash -c 'mkdir -p "$0/$1" && rsync -a --copy-links "$1/" "$0/$1/"' "$dest_dir" {}


# use ghrc.io to avoid dockerhub rate limits
FROM alpine:latest

# Copy the build artifact from the builder stage
COPY --from=builder /tmp/build/target/release/movement-full-node-setup /app/movement-full-node-setup
COPY --from=builder /tmp/runtime/nix/store /nix/store
COPY --from=builder /tmp/runtime/nix/store /nix/store

# Copy celestia-appd and celestia to a location on the alpine path
COPY --from=builder /tmp/executables/celestia-appd /usr/local/bin/celestia-appd
COPY --from=builder /tmp/executables/celestia /usr/local/bin/celestia

# Copy anvil, forge, and solc to a location on the alpine path
# Note: you look this up by running `which anvil` and then `cat $(which anvil)`
COPY --from=builder /tmp/executables/anvil /usr/local/bin/anvil
COPY --from=builder /tmp/executables/forge /usr/local/bin/forge
COPY --from=builder /tmp/executables/solc /usr/local/bin/solc

# Copy over the context so we have the contracts
COPY . /app

# Set the working directory to /app
WORKDIR /app

# Set the binary as the entrypoint
ENTRYPOINT ["/app/movement-full-node-setup"]