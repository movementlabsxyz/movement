{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/2ac40064487f7dfae54f188705e8ed9173993e79";
    rust-overlay.url = "github:oxalica/rust-overlay/db12d0c6ef002f16998723b5dd619fa7b8997086";
    flake-utils.url = "github:numtide/flake-utils";
    foundry.url = "github:shazow/foundry.nix/f533e2c70e520adb695c9917be21d514c15b1c4d"; 
    crane.url = "github:ipetkov/crane";
    crane.inputs.nixpkgs.follows = "nixpkgs";
    
  };

  outputs = { nixpkgs, rust-overlay, flake-utils, foundry, crane, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) foundry.overlay ];
        };

        toolchain = p: (p.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override {
          extensions = [ "rustfmt" "clippy" ];
        };
        craneLib = (crane.mkLib pkgs).overrideToolchain(toolchain);

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
          tesseract4
          ansible
          zlib
          fixDarwinDylibNames
        ];
        
        sysDependencies = with pkgs; [] 
        ++ lib.optionals stdenv.isDarwin [
          frameworks.Security
          frameworks.CoreServices
          frameworks.SystemConfiguration
          frameworks.AppKit
          libelf
        ] ++ lib.optionals stdenv.isLinux [
          udev
          systemd
          snappy
          bzip2
          elfutils
        ];

        testDependencies = with pkgs; [
          python311
          poetry
          just
          foundry-bin
          process-compose
          celestia-node
          celestia-app
          jq
          docker
          solc
          grpcurl
          grpcui
        ];

        # Specific version of toolchain
        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rust;
          rustc = rust;
        };

        celestia-app = pkgs.buildGoModule {
          pname = "celestia-app";
          version = "1.8.0";

          src = pkgs.fetchgit {
            url = "https://github.com/celestiaorg/celestia-app";
            rev = "e75a1fdc8f2386d9f389cb596c88ca7cc19563af";
            hash = "sha256-EE9r1sybbm4Hyh57/nC8utMx/uFdMsIdPecxBtDqAbk=";
          };

          vendorHash = "sha256-2vU1liAm0us7Nk1eawgMvarhq77+IUS0VE61FuvQbuQ=";
          subPackages = [ "cmd/celestia-appd" ];
        };

        celestia-node = pkgs.buildGoModule {
          pname = "celestia-node";
          version = "0.13.3";

          src = pkgs.fetchgit {
            url = "https://github.com/celestiaorg/celestia-node";
            rev = "05238b3e087eb9ecd3b9684cd0125f2400f6f0c7";
            hash = "sha256-bmFcJrC4ocbCw1pew2HKEdLj6+1D/0VuWtdoTs1S2sU=";
          };

          vendorHash = "sha256-8RC/9KiFOsEJDpt7d8WtzRLn0HzYrZ1LIHo6lOKSQxU=";
          subPackages = [ "cmd/celestia" "cmd/cel-key" ];
        };
    
      in {
        packages = {
          inherit celestia-app celestia-node;
        };
        devShells = rec {
          default = docker-build;
          docker-build = pkgs.mkShell {
            ROCKSDB = pkgs.rocksdb;
            SNAPPY = if pkgs.stdenv.isLinux then pkgs.snappy else null;
            OPENSSL_DEV = pkgs.openssl.dev;
         
            buildInputs = with pkgs; [
              # rust toolchain
              (toolchain pkgs)

              # build dependencies
              llvmPackages.bintools openssl openssl.dev libiconv pkg-config
              libclang.lib libz clang pkg-config protobuf rustPlatform.bindgenHook
              lld mold coreutils postgresql

              # test dependencies
              python311 poetry just foundry-bin process-compose jq docker solc
              grpcurl grpcui

              celestia-app celestia-node
            ] ++ lib.optionals stdenv.isDarwin (with pkgs.darwin.apple_sdk.frameworks; [
              Security CoreServices SystemConfiguration AppKit
            ]) ++ lib.optionals stdenv.isLinux (with pkgs; [
              udev systemd snappy bzip2 elfutils.dev
            ]);

            LD_LIBRARY_PATH = "${pkgs.stdenv.cc.cc.lib}/lib/";

            shellHook = ''
              #!/usr/bin/env ${pkgs.bash}

              DOT_MOVEMENT_PATH=$(pwd).movement
              mkdir -p $DOT_MOVEMENT_PATH

              # export PKG_CONFIG_PATH=$PKG_CONFIG_PATH_FOR_TARGET

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
        };
      }
    );
}
