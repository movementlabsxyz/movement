{ pkgs, frameworks, RUSTFLAGS, craneLib }:

let
    # src = craneLib.cleanCargoSource (craneLib.path ./..);
    src = craneLib.path ./..;

    crateName = craneLib.crateNameFromCargoToml { inherit src; };

    # needed to build cargoVendorDir
    baseArgs = {
        inherit src;
    };

    aptosCoreRepoUrl = "https://github.com/movementlabsxyz/aptos-core";
    isAptosCoreRepo = pkgs.lib.any (p: pkgs.lib.hasPrefix ("git+" + aptosCoreRepoUrl)  p.source);

    # derivation override function for applying patches to the `aptos-core` repo
    aptosCoreSrcsOverride = drv: drv.overrideAttrs (_old: {
        # apply a patch to change relative paths in `include_bytes!()` macros
        patches = [
            ./m1-da-light-node-relative-paths.patch
        ];

        # move files to the new relative paths
        postPatch = ''
            cp aptos-move/framework/src/aptos-natives.bpl third_party/move/move-prover/src/
            cp api/doc/{.version,spec.html} crates/aptos-faucet/core/src/endpoints/
            cp aptos-move/move-examples/scripts/minter/build/Minter/bytecode_scripts/main.mv \
                crates/aptos-faucet/core/src/funder/
        '';
    });

    # manually vendor deps with overrides applied
    cargoVendorDir = craneLib.vendorCargoDeps ( baseArgs // {
        overrideVendorGitCheckout = ps: drv: if isAptosCoreRepo ps then aptosCoreSrcsOverride drv else drv;
    });

    commonArgs = baseArgs // {
        strictDeps = true;
        doCheck = false;

        pname = "m1-da-light-node";
        inherit (crateName) version;

        inherit cargoVendorDir;

        nativeBuildInputs = [
            # required for system package discovery
            pkgs.pkg-config
            # required for alternate linkers
            pkgs.clang
            # provides lld linker
            pkgs.llvmPackages.bintools
            # required for protobuf builds
            pkgs.protobuf_26
            # needed by aptos-cached-packages
            pkgs.rustfmt
        ] ++ (pkgs.lib.optionals pkgs.stdenv.isLinux [
            # provides libudev; required for crate `hidapi` and maybe others
            pkgs.systemd
        ]);

        buildInputs = [
            pkgs.openssl
        ] ++ (pkgs.lib.optionals pkgs.stdenv.isDarwin [
            # converts between character encodings on MacOS
            pkgs.libiconv
            # MacOS platform APIs
            pkgs.darwin.IOKit
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
        ]);

        # we have to move the sources recursively to a mutable vendor directory
        # because aptos-cached-packages modifies files outside of `OUT_DIR`.
        # this incurs a performance cost but can only otherwise be fixed
        # upstream.
        # source: https://crane.dev/faq/sandbox-unfriendly-build-scripts.html
        postPatch = ''
            mkdir -p "$TMPDIR/nix-vendor"
            cp -Lr "$cargoVendorDir" -T "$TMPDIR/nix-vendor"
            sed -i "s|$cargoVendorDir|$TMPDIR/nix-vendor/|g" "$TMPDIR/nix-vendor/config.toml"
            chmod -R +w "$TMPDIR/nix-vendor"
            cargoVendorDir="$TMPDIR/nix-vendor"
        '';

        # some crates need direct access to libclang
        LIBCLANG_PATH = "${pkgs.llvmPackages_18.libclang.lib}/lib";
    };

    cargoArtifacts = craneLib.buildDepsOnly commonArgs;
    m1-da-light-node = craneLib.buildPackage (commonArgs // {
        inherit cargoArtifacts;
        cargoExtraArgs = "--package m1-da-light-node";
        doNotRemoveReferencesToVendorDir = true;
    });
in
    m1-da-light-node
