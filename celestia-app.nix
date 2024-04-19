{ pkgs,  }:

pkgs.buildGoModule rec {
  pname = "celestia-app";
  version = "v1.8.0";

  src = pkgs.fetchFromGitHub {
    owner = "celestiaorg";
    repo = "celestia-app";
    rev = "e75a1fdc8f2386d9f389cb596c88ca7cc19563af";
    sha256 = "sha256-YCwIJ55lkLcViVzmAeCIrPtc9mJ/N0eswKrlu9BEC3g=";  # Replace with the actual sha256
  };

  vendorHash = "sha256-UyNNVDO/FFKp80rI5kOI4xfKpkhqF53lgiOSJhCm79U=";  # Replace with the correct vendor hash

  # Specify the subpackage to build
  subPackages = [ "cmd/celestia-appd" ];

  meta = with pkgs.lib; {
    description = "Build specific Go subpackage in Nix";
    homepage = "https://github.com/celestiaorg/celestia-app";
    license = licenses.mit;
    maintainers = with maintainers; [ maintainers.example ];
  };
}