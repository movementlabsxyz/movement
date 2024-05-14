{ pkgs ? import <nixpkgs> {} }:

pkgs.rustPlatform.buildRustPackage rec {
  pname = "forge";
  version = "0.1.0";

  src = pkgs.fetchFromGitHub {
    owner = "foundry-rs";
    repo = "foundry";
    rev = "cafc2606a2187a42b236df4aa65f4e8cdfcea970";
    sha256 = "sha256-EE9r1sybbm4Hyh57/nd8utMx/uFdMsIdPecxBtDqAbk=";
  };

  cargoSha256 = "sha256-EE9r1sybbm4Hyh57/nC8dtMx/uFdMsIdPecxBtDqAbk=";

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  meta = with pkgs.lib; {
    description = "Foundry Forge";
    homepage = "https://github.com/foundry-rs/foundry";
    license = licenses.mit;
    maintainers = [ maintainers.your_name ];
  };
}
