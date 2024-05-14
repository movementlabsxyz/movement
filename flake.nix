{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/f1010e0469db743d14519a1efd37e23f8513d714";
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
    flake-utils.lib.eachSystem ["aarch64-darwin" "x86_64-darwin" "x86_64-linux" "aarch64-linux"] (

      system: let

        overrides = (builtins.fromTOML (builtins.readFile ./rust-toolchain.toml));

        overlays = [(import rust-overlay)];

        pkgs = import nixpkgs {
          inherit system overlays;
        };

        frameworks = pkgs.darwin.apple_sdk.frameworks;

        # celestia-node
        celestia-node = import ./nix/celestia-node.nix { inherit pkgs; };

        # celestia-app
        celestia-app = import ./nix/celestia-app.nix { inherit pkgs; };

        # forge
        forge = import ./nix/forge.nix { inherit pkgs; };

        # anvil
        anvil = import ./nix/anvil.nix { inherit pkgs; };

        # monza-aptos
        monza-aptos = import ./nix/monza-aptos.nix { inherit pkgs; };

        # Specific version of toolchain
        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rust;
          rustc = rust;
        };

        dependencies = with pkgs; [
          forge
          anvil
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
        ];

    
      in
        with pkgs; {

          # Monza Aptos
          packages.monza-aptos = monza-aptos;

          # Development Shell
          devShells.default = mkShell {

            OPENSSL_DEV=pkgs.openssl.dev;
            PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
            buildInputs = dependencies;
            nativeBuildInputs = dependencies;

            shellHook = ''
              #!/bin/bash
              export MONZA_APTOS_PATH=$(nix path-info -r .#monza-aptos | tail -n 1)
              install-foundry
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