{ pkgs, commonArgs, craneLib }:

craneLib.buildPackage (commonArgs // {
    doCheck = false;
    cargoArtifacts = craneLib.buildDepsOnly commonArgs;
    
    buildPhase = ''
    export PATH=$PATH:${pkgs.rustfmt}/bin
    cargo build --release --package aptos-faucet-service
    '';

    installPhase = ''
    mkdir -p $out/bin
    cp target/release/aptos-faucet-service $out/bin/
    '';
})

# {
#   description = "Build the aptos-faucet-service binary";

#   inputs = {
#     nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

#     crane = {
#       url = "github:ipetkov/crane";
#       inputs.nixpkgs.follows = "nixpkgs";
#     };

#     flake-utils.url = "github:numtide/flake-utils";
#   };

#   outputs = { self, nixpkgs, crane, flake-utils, ... }:
#     flake-utils.lib.eachDefaultSystem (system:
#       let
#         pkgs = nixpkgs.legacyPackages.${system};

#         craneLib = crane.mkLib pkgs;
#         frameworks = pkgs.darwin.apple_sdk.frameworks;

#         commonArgs = {
#           src = pkgs.fetchFromGitHub {
#             owner = "movementlabsxyz";
#             repo = "aptos-core";
#             rev = "06443b81f6b8b8742c4aa47eba9e315b5e6502ff";
#             sha256 = "sha256-iIYGbIh9yPtC6c22+KDi/LgDbxLEMhk4JJMGvweMJ1Q=";
#           };
#           strictDeps = true;
          
#           buildInputs = with pkgs; [
#             libiconv 
#             rocksdb
#             rustfmt
#           ] ++ lib.optionals stdenv.isDarwin [
#             frameworks.Security
#             frameworks.CoreServices
#             frameworks.SystemConfiguration
#             frameworks.AppKit
#           ] ++ lib.optionals stdenv.isLinux [
#             udev
#             systemd
#             snappy
#             bzip2
#           ];
#         };

#         aptos-faucet-service = craneLib.buildPackage (commonArgs // {
#           doCheck = false;
#           cargoArtifacts = craneLib.buildDepsOnly commonArgs;
          
#           buildPhase = ''
#             export PATH=$PATH:${pkgs.rustfmt}/bin
#             cargo build --release --package aptos-faucet-service
#           '';

#           installPhase = ''
#             mkdir -p $out/bin
#             cp target/release/aptos-faucet-service $out/bin/
#           '';
#         });
#       in
#       {
#         packages.default = aptos-faucet-service;

#         apps.default = flake-utils.lib.mkApp {
#           drv = aptos-faucet-service;
#         };

#         devShells.default = craneLib.devShell {
#           checks = self.checks.${system};
#         };
#       });
# }
