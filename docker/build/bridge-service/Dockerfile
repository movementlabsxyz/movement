FROM nixos/nix:latest AS builder

RUN nix-env -iA nixpkgs.rsync nixpkgs.glibc nixpkgs.gawk

# Copy the source code into the container
COPY . /tmp/build
WORKDIR /tmp/build

# Build the Rust application
RUN nix --extra-experimental-features "nix-command flakes" \
        develop .#docker-build --command bash -c "cargo build --release -p bridge-service"

RUN rust_binary="./target/release/bridge-service"; dest_dir="/tmp/runtime"; \
    mkdir -p "$dest_dir"; ldd "$rust_binary" | awk '{print $3}' | \
    grep '^/' | xargs -I {} dirname {} | sort | uniq | xargs -I {} \
    bash -c 'mkdir -p "$0/$1" && rsync -a --copy-links "$1/" "$0/$1/"' "$dest_dir" {}

FROM alpine:latest

# Copy the build artifact from the builder stage
COPY --from=builder /tmp/build/target/release/bridge-service /app/bridge-service
COPY --from=builder /tmp/runtime/nix/store /nix/store

# Set the binary as the entrypoint
ENTRYPOINT ["/app/bridge-service"]

