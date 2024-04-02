{ pkgs }:

pkgs.stdenv.mkDerivation {
  name = "celestia-node";
  version = "v0.13.2"; # Update to the desired version

  src = pkgs.fetchFromGitHub {
    owner = "celestiaorg";
    repo = "celestia-node";
    rev = version;
    sha256 = "0000000000000000000000000000000000000000000000000000"; # Replace with the correct hash
  };

  nativeBuildInputs = with pkgs; [
    git
    go
    gnumake
    gcc
    protobuf
    clang
    llvm
    openssl
    rustc
    cargo
  ];

  preBuild = ''
    export HOME=$TMPDIR
    export GOPATH="$TMPDIR/go"
    export GOCACHE="$TMPDIR/go-cache"
    mkdir -p $GOPATH $GOCACHE
  '';

  buildPhase = ''
    make build
  '';

  installPhase = ''
    mkdir -p $out/bin
    cp -r * $out/bin
  '';

  meta = with pkgs.lib; {
    description = "Celestia Node";
    homepage = "https://github.com/celestiaorg/celestia-node";
    license = licenses.mit;
    maintainers = with maintainers; [ ]; # Add maintainers here
  };
}
