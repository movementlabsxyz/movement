{ pkgs ? import <nixpkgs> {} }:

pkgs.rustPlatform.buildRustPackage rec {
  pname = "anvil";
  version = "0.1.0";

  src = pkgs.fetchFromGitHub {
    owner = "foundry-rs";
    repo = "foundry";
    rev = "cafc2606a2187a42b236df4aa65f4e8cdfcea970";
    sha256 = "sha256-sF6Jy27LU14rp5wAxcpjPA5Es5NesJ7Ua3U2vFPjJ+o=";
  };

  cargoSha256 = "sha256-EE9r1sybbm4Hyh57/nCrutMx/uFdMsIdPecxBtDqAbk=";

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  meta = with pkgs.lib; {
    description = "Foundry Anvil";
    homepage = "https://github.com/foundry-rs/foundry";
    license = licenses.mit;
    maintainers = [ maintainers.your_name ];
  };
}
