{ pkgs }:

pkgs.stdenv.mkDerivation rec {
  name = "celestia-node";
  version = "v0.13.2"; # Update to the desired version

  src = pkgs.fetchgit {
    url = "https://github.com/celestiaorg/celestia-node.git";
    rev = version;
    sha256 = "YCwIJ55lkLcViVzmAeCIrPtc9mJ/N0eswKrlu9BEC3g="; 
    leaveDotGit = true;
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
    export HOME=$TMPDIR
    export GOPATH="$TMPDIR/go"
    export GOCACHE="$TMPDIR/go-cache"
    mkdir -p $GOPATH $GOCACHE
    make build && make install
    make cel-key && make install-key
  '';

  installPhase = ''
    mkdir -p $out/bin
    cp $GOPATH/bin/celestia $out/bin
    cp $GOPATH/bin/cel-key $out/bin
  '';


  meta = with pkgs.lib; {
    description = "Celestia Node";
    homepage = "https://github.com/celestiaorg/celestia-node";
    license = licenses.mit;
    maintainers = with maintainers; [ ]; # Add maintainers here
  };
}
