# Nix builder
FROM nixos/nix:latest

# Copy our source and setup our working dir.
COPY . /tmp/build
WORKDIR /tmp/build

RUN nix \
    --extra-experimental-features "nix-command flakes" \
    --option filter-syscalls false \
    develop --command bash  -c "cargo build --release && cd MONZA_APTOS_PATH cargo build --release -p aptos-faucet-service"