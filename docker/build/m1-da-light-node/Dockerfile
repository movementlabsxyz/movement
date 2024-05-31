# Nix builder
FROM nixos/nix:latest

# Copy our source and setup our working dir.
COPY . /app
WORKDIR /app

RUN nix \
    --extra-experimental-features "nix-command flakes" \
    --option filter-syscalls false \
    develop --command bash  -c "cargo build"
