{ pkgs }:

pkgs.stdenv.mkDerivation rec {
  pname = "celestia-node";
  version = "0.13.2";
  commit_hash = "c1b41b0973e9d140b7651295e879d27ad47f42c4";

  src = builtins.fetchGit {
    url = "https://github.com/celestiaorg/celestia-node.git";
    ref = version;
    rev = commit_hash;
  };

  nativeBuildInputs = with pkgs; [
    git go gnumake gcc protobuf clang llvm openssl rustc cargo coreutils
  ];

  preBuild = ''
    export HOME=$TMPDIR
    export GOPATH="$TMPDIR/go"
    export GOCACHE="$TMPDIR/go-cache"
    mkdir -p $GOPATH $GOCACHE
  '';

  buildFlags = [ "build" "cel-key" ];

  installPhase = ''
    mkdir -p $out/bin
    ls -la $TMPDIR/go
    cp $GOPATH/bin/celestia $out/bin/celestia
    cp $GOPATH/bin/cel-key $out/bin/cel-key
  '';

  meta = with pkgs.lib; {
    description = "Celestia Node";
    homepage = "https://github.com/celestiaorg/celestia-node";
    license = licenses.mit;
    maintainers = with maintainers; [ ]; # Add maintainers here
  };
}