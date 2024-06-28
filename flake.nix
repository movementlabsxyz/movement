{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/f1010e0469db743d14519a1efd37e23f8513d714";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    foundry.url = "github:shazow/foundry.nix/monthly"; 
    crane.url = "github:ipetkov/crane";
    crane.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
    foundry,
    crane,
    ...
    }:
    flake-utils.lib.eachSystem ["aarch64-darwin" "x86_64-darwin" "x86_64-linux" "aarch64-linux"] (

      system: let
        overrides = (builtins.fromTOML (builtins.readFile ./rust-toolchain.toml));

        overlays = [
          (import rust-overlay)
          foundry.overlay
        ];

        pkgs = import nixpkgs {
          inherit system overlays;
        };

        craneLib = crane.mkLib pkgs;

        frameworks = pkgs.darwin.apple_sdk.frameworks;

        buildDependencies = with pkgs; [
          llvmPackages.bintools
          openssl
          openssl.dev
          libiconv 
          pkg-config
          libclang.lib
          libz
          clang
          pkg-config
          protobuf
          rustPlatform.bindgenHook
          lld
          mold
          coreutils
          gcc
          rust
          postgresql
        ];
        
        sysDependencies = with pkgs; [] 
        ++ lib.optionals stdenv.isDarwin [
          frameworks.Security
          frameworks.CoreServices
          frameworks.SystemConfiguration
          frameworks.AppKit
        ] ++ lib.optionals stdenv.isLinux [
          udev
          systemd
          snappy
          bzip2
        ];

        testDependencies = with pkgs; [
          just
          foundry-bin
          process-compose
          celestia-node
          celestia-app
          monza-aptos
          jq
          docker
          grpcurl
        ];

        # Specific version of toolchain
        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rust;
          rustc = rust;
        };

        # Needs to be removed soon and replaced with aptos-faucet-service
        monza-aptos = pkgs.stdenv.mkDerivation {
            pname = "monza-aptos";
            version = "branch-monza";

            src = pkgs.fetchFromGitHub {
                owner = "movementlabsxyz";
                repo = "aptos-core";
                rev = "06443b81f6b8b8742c4aa47eba9e315b5e6502ff";
                sha256 = "sha256-iIYGbIh9yPtC6c22+KDi/LgDbxLEMhk4JJMGvweMJ1Q=";
            };

            installPhase = ''
                cp -r . $out
            '';

            meta = with pkgs.lib; {
                description = "Aptos core repository on the monza branch";
                homepage = "https://github.com/movementlabsxyz/aptos-core";
                license = licenses.asl20;
            };
        };
        # Remember, remove this thing above
        
        # celestia-node
        celestia-node = import ./nix/celestia-node.nix { inherit pkgs; };

        # celestia-app
        celestia-app = import ./nix/celestia-app.nix { inherit pkgs; };

        # aptos-faucet-service
        aptos-faucet-service = import ./nix/aptos-faucet-service.nix { 
          inherit pkgs; 
          commonArgs = {
            src = pkgs.fetchFromGitHub {
              owner = "movementlabsxyz";
              repo = "aptos-core";
              rev = "06443b81f6b8b8742c4aa47eba9e315b5e6502ff";
              sha256 = "sha256-iIYGbIh9yPtC6c22+KDi/LgDbxLEMhk4JJMGvweMJ1Q=";
            };
            strictDeps = true;
            
            buildInputs = with pkgs; [] ++buildDependencies ++ sysDependencies;
            nativeBuildInputs = with pkgs; [] ++buildDependencies ++sysDependencies;
          };
          inherit craneLib;
        };
    
      in
        with pkgs; {

          packages.aptos-faucet-service = aptos-faucet-service;

          packages.celestia-node = celestia-node;

          packages.celestia-app = celestia-app;
          
          # Used for workaround for failing vendor dep builds in nix
          devShells.docker-build = mkShell {
            buildInputs = [] ++buildDependencies ++sysDependencies;
            nativeBuildInputs = [] ++buildDependencies ++sysDependencies;
            OPENSSL_DEV=pkgs.openssl.dev;
            PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
            SNAPPY = if stdenv.isLinux then pkgs.snappy else null;
            shellHook = ''
              #!/usr/bin/env bash
              echo "rust-build shell"
            '';
          };

          # Development Shell
          devShells.default = mkShell {

            ROCKSDB=pkgs.rocksdb;
            
            # for linux set SNAPPY variable
            SNAPPY = if stdenv.isLinux then pkgs.snappy else null;

            OPENSSL_DEV = pkgs.openssl.dev;
            PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
            MONZA_APTOS_PATH = monza-aptos;

            buildInputs = [] ++buildDependencies ++sysDependencies ++testDependencies;
            nativeBuildInputs = [] ++buildDependencies ++sysDependencies;

            shellHook = ''
              #!/bin/bash -e
              echo "Monza Aptos path: $MONZA_APTOS_PATH"
              cat <<'EOF'
                 _  _   __   _  _  ____  _  _  ____  __ _  ____
                ( \/ ) /  \ / )( \(  __)( \/ )(  __)(  ( \(_  _)
                / \/ \(  O )\ \/ / ) _) / \/ \ ) _) /    /  )(
                \_)(_/ \__/  \__/ (____)\_)(_/(____)\_)__) (__)
              EOF

              echo "Develop with Move Anywhere"
            '';
          };
        }
    );
}