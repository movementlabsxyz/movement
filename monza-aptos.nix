{ pkgs }:

pkgs.stdenv.mkDerivation {
    pname = "monza-aptos";
    version = "branch-monza";

    src = pkgs.fetchFromGitHub {
        owner = "movementlabsxyz";
        repo = "aptos-core";
        rev = "06443b81f6b8b8742c4aa47eba9e315b5e6502ff";
        sha256 = "sha256-bmFcJrC4ocbCw1pew2HKEdLj6+1D/0VuWtdoTs1S2sU=";
    };

    installPhase = ''
        ls -al && sleep 30
        mkdir -p $out
        cp -r ./* $out/
    '';

    meta = with pkgs.lib; {
        description = "Aptos core repository on the monza branch";
        homepage = "https://github.com/movementlabsxyz/aptos-core";
        license = licenses.asl20;
    };
}
