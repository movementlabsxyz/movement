{ pkgs }:

pkgs.stdenv.mkDerivation {
    pname = "movementswap-core";
    version = "branch-main";

    src = pkgs.fetchFromGitHub {
        owner = "movementlabsxyz";
        repo = "movementswap-core";
        rev = "";
        sha256 = "";
    };

    installPhase = ''
        cp -r . $out
    '';

    meta = with pkgs.lib; {
        description = "Movementswap core repository";
        homepage = "https://github.com/movementlabsxyz/movementswap-core";
        license = licenses.asl20;
    };
}