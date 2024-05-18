# Nix builder
FROM nixos/nix:latest as builder

# Copy our source and setup our working dir.
COPY . /tmp/build
WORKDIR /tmp/build

RUN nix \
    --extra-experimental-features "nix-command flakes" \
    --option filter-syscalls false \
    develop --command bash  -c "cargo build"

# Final image is based on scratch. We copy a bunch of Nix dependencies
# but they're fully self-contained so we don't need Nix anymore.
FROM scratch

WORKDIR /app

# Copy /nix/store
COPY --from=builder /nix/store /nix/store