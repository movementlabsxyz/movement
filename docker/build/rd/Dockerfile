# Use the latest Nix image
FROM nixos/nix:latest

# Set up working directory where the flake will be copied
WORKDIR /app

# Copy the flake files into the container
COPY . /app

# Set the entrypoint to use `nix develop` to execute commands within the Nix environment
ENTRYPOINT ["nix", "develop", "--command", "bash", "-c"]
