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

# Build the Rust application
RUN nix --extra-experimental-features "nix-command flakes" \
        develop .#docker-build --command bash -c "cargo build --release -p m1-da-light-node-celestia-bridge"

RUN rust_binary="./target/release/m1-da-light-node-celestia-bridge"; dest_dir="/tmp/runtime"; \
        mkdir -p "$dest_dir"; ldd "$rust_binary" | awk '{print $3}' | \
        grep '^/' | xargs -I {} dirname {} | sort | uniq | xargs -I {} \
        bash -c 'mkdir -p "$0/$1" && rsync -a --copy-links "$1/" "$0/$1/"' "$dest_dir" {}

# use ghrc.io to avoid dockerhub rate limits
FROM alpine:latest

# Copy the build artifact from the builder stage
COPY --from=builder /tmp/build/target/release/m1-da-light-node-celestia-bridge /app/m1-da-light-node-celestia-bridge
COPY --from=builder /tmp/runtime/nix/store /nix/store

# Copy celestia-appd and celestia to a location on the alpine path
COPY --from=builder /tmp/executables/celestia-appd /usr/local/bin/celestia-appd
COPY --from=builder /tmp/executables/celestia /usr/local/bin/celestia

# Set the binary as the entrypoint
ENTRYPOINT ["/app/m1-da-light-node-celestia-bridge"]