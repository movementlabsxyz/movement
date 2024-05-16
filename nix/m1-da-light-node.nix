{ pkgs, frameworks, RUSTFLAGS }:

pkgs.rustPlatform.buildRustPackage rec {
    pname = "m1-da-light-node";
    version = "0.1.0";

    buildInput = with pkgs; [
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
    ] ++ lib.optionals stdenv.isDarwin [
        frameworks.Security
        frameworks.CoreServices
        frameworks.SystemConfiguration
        frameworks.AppKit
    ] ++ lib.optionals stdenv.isLinux [
        udev
        systemd
    ];

    src = builtins.filterSource
  (path: type: baseNameOf path != ".git")
  ./..; # Use the current directory as the source

    cargoSha256 = pkgs.lib.fakeSha256;

    buildPhase = ''
        # export HOME=$(mktemp -d)
        # export RUSTFLAGS="${RUSTFLAGS}"
        cat .cargo/config.toml
        cargo build --release
    '';

    cargoLock = {
        lockFile = ../Cargo.lock;
        outputHashes = {
            "abstract-domain-derive-0.1.0" = "sha256-53ObE7yoEMuZWjIAXXAm4hDBBKU1VhgEj/Zc9EQ4MBA=";
            "bcs-0.1.4" = "sha256-SzODBDLSQRXExjke0/7FN/wQQq3vxcwFeGOa37H3Gtg=";
            "celestia-proto-0.2.0" = "sha256-/GucnpYoCqQ0plcDTwuUoZxC3rLsNnso1LVTg+tY4+k=";
            "merlin-3.0.0" = "sha256-JATfmaS1EA33gcGJFNzUEZM1pBKh22q0jubA2dtLS1I=";
            "poseidon-ark-0.0.1" = "sha256-xDff0iC/OOzrb3fEycfmb0Rb/omCuVjNoibDOrr/32A=";
            "serde-generate-0.20.6" = "sha256-Oa9inyiTPQv1ScSQog+Ry+c7aLnAx4GcGn5ravGPthM=";
            "sha2-0.10.8" = "sha256-vuFQFlbDXEW+n9+Nx2VeWanggCSd6NZ+GVEDFS9qZ2M=";
            "x25519-dalek-1.2.0" = "sha256-AHjhccCqacu0WMTFyxIret7ghJ2V+8wEAwR5L6Hy1KY=";
            "zstd-sys-2.0.9+zstd.1.5.5" = "sha256-n7abNAHEfDeRSjhh7SpI/BpkJCVLONJwKvaXwVB4PXs=";
        };
        allowBuiltinFetchGit = true;
    };

    meta = with pkgs.lib; {
        description = "M1 DA Light Node";
        homepage = "https://github.com/movementlabsxyz/movement";
        license = licenses.mit;
        maintainers = [ maintainers.your_name ];
    };

}
