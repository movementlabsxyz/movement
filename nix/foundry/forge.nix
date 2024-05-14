{ pkgs }:

pkgs.rustPlatform.buildRustPackage rec {
  pname = "forge";
  version = "0.1.0";

  nativeBuildInputs = [ pkgs.pkg-config ];
  buildInputs = with pkgs; [
    openssl
    libiconv
    clang
    libz
    llvmPackages.libclang
    libusb1
  ] ++ lib.optionals stdenv.isDarwin [
    pkgs.darwin.apple_sdk.frameworks.Security
    pkgs.darwin.apple_sdk.frameworks.CoreServices
  ];

  src = pkgs.fetchFromGitHub {
    owner = "foundry-rs";
    repo = "foundry";
    rev = "cafc2606a2187a42b236df4aa65f4e8cdfcea970";
    sha256 = "sha256-sF6Jy27LU14rp5wAxcpjPA5Es5NesJ7Ua3U2vFPjJ+o=";
  };

  cargoSha256 = "sha256-EE9r1sybbm4Hyh57/nC8dtMx/uFdMsIdPecxBtDqAbk=";

  cargoLock = {
    lockFile = ./Cargo.lock;
    outputHashes = {
      "alloy-consensus-0.1.0" = "sha256-wvxh/AuI2/iMXdkIPrtnixA/56nyDHiBqxR23gfPCD0=";
      "revm-inspectors-0.1.0" = "sha256-69PMxyMUpnQC+GaKrwUJZ6kWCwB9kiyCmtlePyxanAI=";
    };
    allowBuiltinFetchGit = true;
  };

  meta = with pkgs.lib; {
    description = "Foundry Forge";
    homepage = "https://github.com/foundry-rs/foundry";
    license = licenses.mit;
    maintainers = [ maintainers.your_name ];
  };
}
