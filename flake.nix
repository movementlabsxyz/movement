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
    flake-utils.lib.eachSystem ["aarch64-darwin" "x86_64-linux" "aarch64-linux"] (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        frameworks = pkgs.darwin.apple_sdk.frameworks;

        # celestia-node
        celestia-node = import ./celestia-node.nix { inherit pkgs; };

        # celestia-app
        celestia-app = import ./celestia-app.nix { inherit pkgs; };

        # monza-aptos
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
       
        # Specific version of toolchain
        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rust;
          rustc = rust;
        };

        runtimeDependencies = with pkgs; [
          openssl
          openssl.dev
          libiconv 
          pkg-config
          process-compose
          just
          jq
        ] ++ lib.optionals stdenv.isDarwin [

        ] ++ lib.optionals stdenv.isLinux [

        ];


        buildDependencies = with pkgs; [
            libclang.lib
            libz
            clang
            pkg-config
            protobuf
            rustPlatform.bindgenHook
            lld
            coreutils
            gcc
          ]
          ++ runtimeDependencies
          # Be it Darwin
          ++ lib.optionals stdenv.isDarwin [
            frameworks.Security
            frameworks.CoreServices
            frameworks.SystemConfiguration
            frameworks.AppKit
          ]
          ++ lib.optionals stdenv.isLinux [
            systemd
          ];

        testingDependencies = with pkgs; [
            celestia-node
            celestia-app
            monza-aptos
        ]
        ++ buildDependencies;

        developmentDependencies = with pkgs; [
          rust
        ] ++ testingDependencies;

    
      in
        with pkgs; {

          # Monza Aptos
          packages.monza-aptos = monza-aptos;

          # Development Shell
          devShells.default = mkShell {

            OPENSSL_DEV=pkgs.openssl.dev;
            PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
            buildInputs = developmentDependencies;

            environment.systemPackages = with pkgs; [
              openssl
            ];

            environment.variables = {
              PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig";
            };

            # PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";

            shellHook = ''
              #!/bin/bash
              export LD_LIBRARY_PATH=${lib.getLib gcc}/lib:${lib.getLib stdenv.cc.cc.lib}/lib:$LD_LIBRARY_PATH
              export MONZA_APTOS_PATH=$(nix path-info -r .#monza-aptos | tail -n 1)
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