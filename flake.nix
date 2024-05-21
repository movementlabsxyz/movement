{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/f1010e0469db743d14519a1efd37e23f8513d714";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    foundry.url = "github:shazow/foundry.nix/monthly"; 
    naersk.url = "github:nix-community/naersk";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
    foundry,
    naersk,
    ...
    }:
    flake-utils.lib.eachSystem ["aarch64-darwin" "x86_64-darwin" "x86_64-linux" "aarch64-linux"] (

      system: let

        # nix does not handle .cargo/config.toml
        RUSTFLAGS = if pkgs.stdenv.hostPlatform.isLinux then
          "--cfg tokio_unstable -C force-frame-pointers=yes -C force-unwind-tables=yes -C link-arg=-fuse-ld=lld -C target-feature=+sse4.2"
        else if pkgs.stdenv.hostPlatform.isWindows then
          "--cfg tokio_unstable -C force-frame-pointers=yes -C force-unwind-tables=yes -C link-arg=/STACK:8000000"
        else
          "--cfg tokio_unstable -C force-frame-pointers=yes -C force-unwind-tables=yes";

        overrides = (builtins.fromTOML (builtins.readFile ./rust-toolchain.toml));

        overlays = [
          (import rust-overlay)
          foundry.overlay
        ];

        pkgs = import nixpkgs {
          inherit system overlays;
        };

        frameworks = pkgs.darwin.apple_sdk.frameworks;

         dependencies = with pkgs; [
          rocksdb
          foundry-bin
          # solc
          llvmPackages.bintools
          openssl
          openssl.dev
          libiconv 
          pkg-config
          process-compose
          just
          jq
          libclang.lib
          libz
          clang
          pkg-config
          protobuf
          rustPlatform.bindgenHook
          lld
          coreutils
          gcc
          rust
          celestia-node
          celestia-app
          monza-aptos
        ] ++ lib.optionals stdenv.isDarwin [
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

        # Specific version of toolchain
        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rust;
          rustc = rust;
        };

        naersk' = pkgs.callPackage naersk {
          cargo = rust;
          rustc = rust;
        };

        # celestia-node
        celestia-node = import ./nix/celestia-node.nix { inherit pkgs; };

        # celestia-app
        celestia-app = import ./nix/celestia-app.nix { inherit pkgs; };

        # monza-aptos
        monza-aptos = import ./nix/monza-aptos.nix { inherit pkgs; };

        # m1-da-light-node
        m1-da-light-node = import ./nix/m1-da-light-node.nix { inherit pkgs frameworks RUSTFLAGS; };
    
      in
        with pkgs; {

          # Monza Aptos
          packages.monza-aptos = monza-aptos;

          # M1 DA Light Node
          packages.m1-da-light-node = m1-da-light-node;

          # Development Shell
          devShells.default = mkShell {

            ROCKSDB=pkgs.rocksdb;
            
            # for linux set SNAPPY variable
            SNAPPY = if stdenv.isLinux then pkgs.snappy else null;

            OPENSSL_DEV=pkgs.openssl.dev;
            PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
            buildInputs = dependencies;
            nativeBuildInputs = dependencies;

            shellHook = ''
              #!/usr/bin/env bash
              export MONZA_APTOS_PATH=$(nix path-info -r .#monza-aptos | tail -n 1)
              echo "Monza Aptos Path: $MONZA_APTOS_PATH"
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