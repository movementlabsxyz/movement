{ pkgs, commonArgs, craneLib }:

craneLib.buildPackage (commonArgs // {
    doCheck = false;
    cargoArtifacts = craneLib.buildDepsOnly commonArgs;
    
    buildPhase = ''
        export PATH=$PATH:${pkgs.rustfmt}/bin
        export OPENSSL_DEV=${pkgs.openssl.dev}
        export PKG_CONFIG_PATH=${pkgs.openssl.dev}/lib/pkgconfig${pkgs.lib.optionalString pkgs.stdenv.isLinux ":${pkgs.lib.getDev pkgs.systemd}/lib/pkgconfig"}
        export SNAPPY=${if pkgs.stdenv.isLinux then pkgs.snappy else ""}
        export PATH=$PATH:${pkgs.rustfmt}/bin
        cargo build --release --package aptos-faucet-service
    '';

    installPhase = ''
        mkdir -p $out/bin
        cp target/release/aptos-faucet-service $out/bin/
    '';
})
