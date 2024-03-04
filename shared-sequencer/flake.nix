{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
    ...
    }:
    flake-utils.lib.eachSystem ["aarch64-darwin" "x86_64-linux"] (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        frameworks = pkgs.darwin.apple_sdk.frameworks;

        # Include avalanche network runner and avalanchego
        avalanche-network-runner = import ./avalanche-network-runner.nix { inherit pkgs; };
        avalanchego = with pkgs; callPackage ./avalanchego.nix {
            IOKit = lib.optionals pkgs.stdenv.isDarwin frameworks.IOKit;
        };

        # Specific version of toolchain
        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain;

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rust;
          rustc = rust;
        };

        runtimeDependencies = with pkgs; [
          openssl
        ];


        buildDependencies = with pkgs; [
            libclang.lib
            libz
            clang
            pkg-config
            protobuf
            rustPlatform.bindgenHook]
          ++ runtimeDependencies
          # Be it Darwin
          ++ lib.optionals stdenv.isDarwin [
            frameworks.Security
            frameworks.CoreServices
            frameworks.SystemConfiguration
            frameworks.AppKit
          ];

        testingDependencies = with pkgs; [
            avalanchego
            avalanche-network-runner
        ]
        ++ buildDependencies;

        developmentDependencies = with pkgs; [
            rust
          ]
          ++ testingDependencies;

        movement-sequencer-cargo-toml = builtins.fromTOML (builtins.readFile ./sequencer/Cargo.toml);
      in
        with pkgs; {
          packages = flake-utils.lib.flattenTree rec {
            movement-sequencer = rustPlatform.buildRustPackage {
              pname = movement-sequencer-cargo-toml.package.name;
              version = movement-sequencer-cargo-toml.package.version;

              env = { LIBCLANG_PATH = "${libclang.lib}/lib"; }
              // (lib.optionalAttrs (stdenv.cc.isClang && stdenv.isDarwin) { NIX_LDFLAGS = "-l${stdenv.cc.libcxx.cxxabi.libName}"; });

              src = ./.;
              cargoLock = {
                lockFile = ./Cargo.lock;
              };

              nativeBuildInputs = testingDependencies;
              buildInputs = runtimeDependencies;
              # false would skip the tests
              doCheck = true;
              preCheck = ''
                  export VM_PLUGIN_PATH=$out/bin/sequencer
                '';
            };

            default = movement-sequencer;
          };

          # Development Shell
          devShells.default = mkShell {
            buildInputs = developmentDependencies;

            shellHook = ''
              ${lib.optionalString (stdenv.cc.isClang && stdenv.isDarwin) "export NIX_LDFLAGS='-l${stdenv.cc.libcxx.cxxabi.libName}'"}
              export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"
              echo NB. Plugin is built as debug
              export VM_PLUGIN_PATH=$(pwd)/target/debug/sequencer
              #!/bin/bash

              cat <<'EOF'
                 _  _   __   _  _  ____  _  _  ____  __ _  ____
                ( \/ ) /  \ / )( \(  __)( \/ )(  __)(  ( \(_  _)
                / \/ \(  O )\ \/ / ) _) / \/ \ ) _) /    /  )(
                \_)(_/ \__/  \__/ (____)\_)(_/(____)\_)__) (__)
              EOF

              echo "Develop with Move Anywhere"

              read -p "Do you want to clean the build directory? [y/N]: " clean_reply
              if [[ $clean_reply =~ ^[Yy]$ ]]; then
                echo "Cleaning build directory..."
                cargo clean
              fi

              if [ -f "$VM_PLUGIN_PATH" ]; then
                echo "VM_PLUGIN_PATH="$VM_PLUGIN_PATH
              else
                echo "Building plugin..."
                cargo build
              fi
              mkdir -p data
              export AVALANCHEGO_DATA_DIR=$(pwd)/data
              export AVALANCHEGO_PATH=${avalanchego}/bin/avalanchego
            '';
          };
        }
    );
}
