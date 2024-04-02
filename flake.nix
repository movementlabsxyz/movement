{
  description = "A flake for the Movment Labs SDK with dependencies covering all modules";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
        celestiaSrc = pkgs.fetchFromGitHub {
          owner = "celestiaorg";
          repo = "celestia-node";
          rev = "v0.13.2";
          # This is a placeholder; you would need to replace it with the actual hash.
          # However, since you prefer not to specify it, you might consider other approaches for development purposes.
          sha256 = "sha256-YCwIJ55lkLcViVzmAeCIrPtc9mJ/N0eswKrlu9BEC3g="; 
        };
      in
      {
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            git
            go
            gnumake
            gcc
            protobuf
            clang
            llvm
            openssl
            rustc
            cargo
            (pkgs.stdenv.mkDerivation {
              name = "celestia";
              src = celestiaSrc;
              buildInputs = with pkgs; [ git go gnumake gcc protobuf clang llvm openssl rustc cargo ];
              buildPhase = ''
                make build
              '';
              installPhase = ''
                mkdir -p $out/bin
                cp -r * $out/bin
              '';
            })
          ];
        };
      }
    );
}
