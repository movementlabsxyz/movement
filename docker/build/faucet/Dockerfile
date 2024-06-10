# Nix builder
FROM nixos/nix:latest AS builder

RUN nix-env -iA nixpkgs.rsync nixpkgs.glibc nixpkgs.gawk

# Copy our source and setup our working dir.
COPY . /tmp/build
WORKDIR /tmp/build

# Build our Nix environment
RUN nix \
    --extra-experimental-features "nix-command flakes" \
    --option filter-syscalls false \
    build .#aptos-faucet-service

# fail quietly, another small workaround, to keep the image small
RUN rust_binary="./result/bin/aptos-faucet-service"; dest_dir="/tmp/runtime"; \
    mkdir -p "$dest_dir"; ldd "$rust_binary" | awk '{print $3}' | \
    grep '^/' | xargs -I {} dirname {} | sort | uniq | xargs -I {} \
    bash -c 'mkdir -p "$0/$1" && rsync -a --copy-links --exclude "*/environment.d/99-environment.conf" "$1/" "$0/$1/"' "$dest_dir" {} || true

FROM scratch

# Copy the build artifact from the builder stage
COPY --from=builder /tmp/build/result/bin/aptos-faucet-service /app/aptos-faucet-service
COPY --from=builder /tmp/runtime/nix/store /nix/store

# Set the binary as the entrypoint
ENTRYPOINT ["/app/aptos-faucet-service"]