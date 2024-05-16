# Nix builder
FROM nixos/nix:latest AS builder

# Copy our source and setup our working dir.
COPY . /tmp/build
WORKDIR /tmp/build

RUN nix \
    --extra-experimental-features "nix-command flakes" \
    --option filter-syscalls false \
    develop --command bash  -c "cargo build --release"

RUN chmod -R 755 . 
CMD ["nix", "--extra-experimental-features", "nix-command flakes", "--option", "filter-syscalls", "false", "develop", "--command", "bash", "-c", "just monza-full-node local -t=false"]
