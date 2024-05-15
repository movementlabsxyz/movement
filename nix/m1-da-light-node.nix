{ pkgs }:

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

    src = ./..;  # Use the current directory as the source

    cargoSha256 = pkgs.lib.fakeSha256;

    meta = with pkgs.lib; {
        description = "M1 DA Light Node";
        homepage = "https://github.com/movementlabsxyz/movement";
        license = licenses.mit;
        maintainers = [ maintainers.your_name ];
    };

}
