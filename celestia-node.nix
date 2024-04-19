{ pkgs }:

let
  celestiaBuild = pkgs.buildFHSUserEnv {
    name = "celestia-node-env";
    targetPkgs = pkgs: (with pkgs; [
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
      # Add any other dependencies or libraries needed
    ]);

    runScript = "bash";
    profile = ''
      export HOME=${pkgs.stdenv.buildPackages.coreutils}/bin
      export GOPATH="$HOME/go"
      export GOCACHE="$HOME/go-cache"
      mkdir -p $GOPATH $GOCACHE
    '';

    multiBuild = true; # This option is crucial for executing multiple build commands

    buildInputs = [
      pkgs.makeWrapper
    ];

    builder = ''
      source $stdenv/setup

      wrapProgram $out/bin/bash \
        --set HOME $HOME \
        --set GOPATH $GOPATH \
        --set GOCACHE $GOCACHE

      echo "Running the Celestia Node build environment..."
      $out/bin/bash -c ''
        source ${profile}
        
        cd ${celestia-src}
        patchShebangs .
        
        make build && make install
        make cel-key && make install-key
        
        mkdir -p $out/bin
        cp $GOPATH/bin/celestia $out/bin
        cp $GOPATH/bin/cel-key $out/bin
      ''
    '';
  };

  celestia-src = pkgs.fetchFromGitHub {
    owner = "celestiaorg";
    repo = "celestia-node";
    rev = "c1b41b0973e9d140b7651295e879d27ad47f42c4";
    sha256 = "0x123...";  # You need to replace this with the correct hash
  };
in
celestiaBuild
