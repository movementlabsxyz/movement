# allow our nixpkgs import to be overridden if desired
{ pkgs ? import <nixpkgs> {} }:

# let's write an actual basic derivation
# this uses the standard nixpkgs mkDerivation function
pkgs.stdenv.mkDerivation {

  # name of our derivation
  name = "movementswap-load-soak-testing";

  # sources that will be used for our derivation.
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
        description = "Aptos core repository on the monza branch";
        homepage = "https://github.com/movementlabsxyz/movementswap-core";
        license = licenses.asl20;
    };
}
