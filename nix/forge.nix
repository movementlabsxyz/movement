{ pkgs ? import <nixpkgs> {} }:

pkgs.rustPlatform.buildRustPackage rec {
  pname = "forge";
  version = "0.1.0";

  src = pkgs.fetchFromGitHub {
    owner = "foundry-rs";
    repo = "foundry";
    rev = "cafc2606a2187a42b236df4aa65f4e8cdfcea970";
    sha256 = "sha256_hash_of_the_source";
  };

  cargoSha256 = "sha256_hash_of_cargo_dependencies";

  meta = with pkgs.lib; {
    description = "Foundry Forge";
    homepage = "https://github.com/foundry-rs/foundry";
    license = licenses.mit;
    maintainers = [ maintainers.your_name ];
  };
}
