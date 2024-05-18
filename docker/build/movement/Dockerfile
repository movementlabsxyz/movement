# Nix builder
FROM nixos/nix:latest as builder

# Copy our source and setup our working dir.
COPY . /tmp/build
WORKDIR /tmp/build

RUN nix \
    --extra-experimental-features "nix-command flakes" \
    --option filter-syscalls false \
    develop --command bash  -c "cargo build --release"

# Final image still runs on nix os, but only contains the /nix/store
FROM nixos/nix:latest

WORKDIR /app

# Copy /nix/store
COPY --from=builder /nix/store /nix/store
COPY --from=builder /tmp/build /app
