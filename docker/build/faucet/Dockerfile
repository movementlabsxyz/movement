# Nix builder
FROM nixos/nix:latest as builder

RUN mkdir -p /etc/nix && echo 'experimental-features = nix-command flakes' > /etc/nix/nix.conf

# Copy our source and setup our working dir.
COPY . /tmp/build
WORKDIR /tmp/build

RUN nix develop .#faucet-build --command bash  -c "mkdir /tmp/monza-aptos && \
    cp -r $MONZA_APTOS_PATH/* /tmp/monza-aptos/ && cd /tmp/monza-aptos && \
    RUSTFLAGS=\"--cfg tokio_unstable\" cargo build --release -p aptos-faucet-service"

# RUN rust_binary="./target/release/aptos-faucet-service"; dest_dir="/tmp/runtime"; \
#     mkdir -p "$dest_dir"; ldd "$rust_binary" | awk '{print $3}' | \
#     grep '^/' | xargs -I {} dirname {} | sort | uniq | xargs -I {} \
#     bash -c 'mkdir -p "$0/$1" && rsync -a --copy-links "$1/" "$0/$1/"' "$dest_dir" {}

# FROM scratch

# # Copy the build artifact from the builder stage
# COPY --from=builder /tmp/build/target/release/aptos-faucet-service /app/aptos-faucet-service
# COPY --from=builder /tmp/runtime/nix/store /nix/store

# # Set the binary as the entrypoint
# ENTRYPOINT ["/app/aptos-faucet-service"]

