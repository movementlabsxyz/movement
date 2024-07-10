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
# Note: you look this up by running `which anvil` and then `cat $(which anvil)`
COPY --from=builder /nix/store/93haq4cas597hmsmvq3g59rjf02c5frw-bash-5.2p26/bin/bash /nix/store/93haq4cas597hmsmvq3g59rjf02c5frw-bash-5.2p26/bin/bash
COPY --from=builder /nix/store/5ihxx11dj3diyi7n7xy463r34ks5s1q1-git-2.44.0/bin /nix/store/5ihxx11dj3diyi7n7xy463r34ks5s1q1-git-2.44.0/bin
COPY --from=builder /nix/store/w0sbla9hmbrnf8qr6ca7b6f3wfaqxxij-foundry-0.0.0/bin /nix/store/w0sbla9hmbrnf8qr6ca7b6f3wfaqxxij-foundry-0.0.0/bin
ENV PATH="/nix/store/w0sbla9hmbrnf8qr6ca7b6f3wfaqxxij-foundry-0.0.0/bin:${PATH}"

# Copy over the context so we have the contracts
COPY . /app

# Set the working directory to /app
WORKDIR /app

# Set the binary as the entrypoint
ENTRYPOINT ["/app/suzuka-full-node-setup"]