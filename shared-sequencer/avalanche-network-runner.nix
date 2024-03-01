{ pkgs ? import <nixpkgs> {} }:

pkgs.buildGoModule rec {
  pname = "avalanche-network-runner";
  version = "1.7.5";

  src = pkgs.fetchFromGitHub {
    owner = "ava-labs";
    repo = pname;
    rev = "v${version}";
    sha256 = "A54KNB9BGKvGp2UsP46U5HteiCOOKrnYatDXUAc/BIg=";
  };

  vendorHash = null;
  vendor = true;

  nativeBuildInputs = with pkgs; [git cacert blst];
  buildInputs = with pkgs; [cacert blst];

  buildPhase = ''
    export GOPROXY=direct
    go build -v -ldflags="-X 'github.com/ava-labs/avalanche-network-runner/cmd.Version=${version}'" -mod=readonly
  '';

  installPhase = ''
    install -Dm755 ./avalanche-network-runner $out/bin/avalanche-network-runner
  '';
}
