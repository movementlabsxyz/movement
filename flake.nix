{
  description = "A flake for the Movment Labs SDK with dependencies covering all modules";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
        celestiaSrc =  pkgs.fetchgit {
          url = "https://github.com/celestiaorg/celestia-node.git";
          rev = "v0.13.2"; # Use ref for tags or branches
          sha256 = "Sxw4ccHiO3nszd4L5wsBXk4MReFfHnzISjRlmJy5KBY=";
          leaveDotGit = true; # Leave the .git directory in the source
          # `rev` and `sha256` are omitted to demonstrate fetching the latest commit on the ref.
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
              preBuild = ''
                export HOME=$TMPDIR
                export GOPATH="$TMPDIR/go"
                export GOCACHE="$TMPDIR/go-cache"
                mkdir -p $GOPATH $GOCACHE
              '';
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
