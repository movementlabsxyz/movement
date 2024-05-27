{ pkgs, frameworks, RUSTFLAGS, craneLib }:

pkgs.stdenv.mkDerivation {
    pname = "m1-da-light-node";
    version = "0.1.0";

     src = craneLib.cleanCargoSource (craneLib.path ./.);

    # Common arguments can be set here to avoid repeating them later
    commonArgs = {
        inherit src;
        strictDeps = true;

        buildInputs = [
    
        ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
 
            pkgs.libiconv
        ];

    };

    cargoArtifacts = craneLib.buildDepsOnly commonArgs;

    individualCrateArgs = commonArgs // {
        inherit cargoArtifacts;
        inherit (craneLib.crateNameFromCargoToml { inherit src; }) version;
        doCheck = false;
    };

    fileSetForCrate = crate: pkgs.lib.fileset.toSource {
        root = ./.;
        fileset = pkgs.lib.fileset.unions [
            ./Cargo.toml
            ./Cargo.lock
            crate
        ];
    };

    m1-da-light-node = craneLib.buildPackage (individualCrateArgs // {
        pname = "m1-da-light-node";
        cargoExtraArgs = "-p m1-da-light-node";
        src = fileSetForCrate ./protocol-units/da/m1/light-node;
    });

    meta = with pkgs.lib; {
        description = "M1 DA Light Node";
        homepage = "https://github.com/movementlabsxyz/movement";
        license = licenses.mit;
        maintainers = [ maintainers.your_name ];
    };

}
