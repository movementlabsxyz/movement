{ pkgs,  }:

pkgs.buildGoModule rec {
  pname = "celestia-node";
  version = "0.13.3";

  src = pkgs.fetchFromGitHub {
    owner = "celestiaorg";
    repo = "celestia-node";
    rev = "05238b3e087eb9ecd3b9684cd0125f2400f6f0c7";
    sha256 = "sha256-bmFcJrC4ocbCw1pew2HKEdLj6+1D/0VuWtdoTs1S2sU=";  # Replace with the actual sha256
  };

  vendorHash = "sha256-8RC/9KiFOsEJDpt7d8WtzRLn0HzYrZ1LIHo6lOKSQxU=";  # Replace with the correct vendor hash

  # Specify the subpackage to build
  subPackages = [ "cmd/celestia" "cmd/cel-key" ];

  meta = with pkgs.lib; {
    description = "Build specific Go subpackage in Nix";
    homepage = "https://github.com/celestiaorg/celestia-node";
    license = licenses.mit;
    maintainers = with maintainers; [ maintainers.example ];
  };
}
