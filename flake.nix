{
  description = "A flake for the Movment Labs SDK with dependencies covering all modules";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, rust-overlay, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
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
            
          ];
        };
      }
    );
}
