{ pkgs,  }:

pkgs.buildGoModule rec {
  pname = "celestia-node";
  version = "0.13.2";

  src = pkgs.fetchFromGitHub {
    owner = "celestiaorg";
    repo = "celestia-node";
    rev = "c1b41b0973e9d140b7651295e879d27ad47f42c4";
    sha256 = "sha256-YCwIJ55lkLcViVzmAeCIrPtc9mJ/N0eswKrlu9BEC3g=";  # Replace with the actual sha256
  };

  vendorHash = "sha256-UyNNVDO/FFKp80rI5kOI4xfKpkhqF53lgiOSJhCm79U=";  # Replace with the correct vendor hash

  # Specify the subpackage to build
  subPackages = [ "cmd/celestia" "cmd/cel-key" ];

  meta = with pkgs.lib; {
    description = "Build specific Go subpackage in Nix";
    homepage = "https://github.com/celestiaorg/celestia-node";
    license = licenses.mit;
    maintainers = with maintainers; [ maintainers.example ];
  };
}
