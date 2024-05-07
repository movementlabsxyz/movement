{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/6143fc5eeb9c4f00163267708e26191d1e918932";
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
          process-compose
          just
          jq
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
            buildInputs = developmentDependencies;

            shellHook = ''
              #!/bin/bash
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