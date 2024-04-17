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
        celestia-node = import ./celestia-node.nix { inherit pkgs; };
       
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
            celestia-node
        ]
        ++ buildDependencies;

        developmentDependencies = with pkgs; [
          rust
        ] ++ testingDependencies;

    
      in
        with pkgs; {

          # Development Shell
          devShells.default = mkShell {
            buildInputs = developmentDependencies;

            shellHook = ''
              #!/bin/bash
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