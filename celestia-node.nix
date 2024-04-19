{ pkgs }:

pkgs.stdenv.mkDerivation rec {
  name = "celestia-node";
  version = "v0.13.2"; # Update to the desired version
  commit_hash = "c1b41b0973e9d140b7651295e879d27ad47f42c4";

  src = builtins.fetchGit {
    url = "https://github.com/celestiaorg/celestia-node.git";
    ref = version;
    rev = commit_hash;
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
    coreutils
  ];

  preBuild = ''
    export HOME=$TMPDIR
    export GOPATH="$TMPDIR/go"
    export GOCACHE="$TMPDIR/go-cache"
    mkdir -p $GOPATH $GOCACHE
    patchShebangs .
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
