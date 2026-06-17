{
  lib,
  stdenv,
  fetchurl,
  autoPatchelfHook,
  versionCheckHook,
  nix-update-script,
}:
let
  version = "0.87.0";
  platform =
    if stdenv.hostPlatform.isLinux && stdenv.hostPlatform.isx86_64 then
      {
        suffix = "amd64-linux";
        hash = "sha256-gptxQocIX/RWK6LGT5yBKEY2UNgzvdHbXVozRx3NZ8s=";
      }
    else
      throw "Unsupported platform: ${stdenv.hostPlatform.system}";
in
stdenv.mkDerivation {
  pname = "barretenberg";
  inherit version;

  src = fetchurl {
    url = "https://github.com/AztecProtocol/aztec-packages/releases/download/v${version}/barretenberg-${platform.suffix}.tar.gz";
    hash = platform.hash;
  };

  nativeBuildInputs = lib.optionals stdenv.isLinux [ autoPatchelfHook ];

  buildInputs = lib.optionals stdenv.isLinux [
    stdenv.cc.cc.lib # libstdc++.so.6 and libgcc_s.so.1
  ];

  sourceRoot = ".";

  installPhase = ''
    mkdir -p $out/bin
    cp bb $out/bin/
  '';

  nativeInstallCheckInputs = [ versionCheckHook ];
  versionCheckProgram = "${placeholder "out"}/bin/bb";
  versionCheckProgramArg = "--version";
  doInstallCheck = true;

  passthru = {
    updateScript = nix-update-script { };
  };

  meta = with lib; {
    description = "Optimized elliptic curve library and PLONK SNARK prover";
    homepage = "https://github.com/AztecProtocol/aztec-packages/tree/master/barretenberg";
    license = licenses.mit;
    mainProgram = "bb";
    platforms = [
      "x86_64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ];
    maintainers = [ ];
    sourceProvenance = [ sourceTypes.binaryNativeCode ];
  };
}
