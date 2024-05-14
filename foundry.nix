{ pkgs,  }:

pkgs.stdenv.mkDerivation rec {
    pname = "foundry";
    version = "latest";

    buildCommand = ''
    mkdir -p $out/bin
    echo "#!${pkgs.stdenv.shell}" > $out/bin/install-foundry
    echo "curl -L https://foundry.paradigm.xyz | bash" >> $out/bin/install-foundry
    echo "foundryup" >> $out/bin/install-foundry
    chmod +x $out/bin/install-foundry
    '';

    meta = with pkgs.lib; {
        description = "Setup script for Foundry, a smart contract development toolchain";
        homepage = "https://github.com/foundry-rs/foundry";
        license = licenses.mit;
        maintainers = with maintainers; [ maintainers.example ];
    };
}
