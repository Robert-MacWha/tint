{
  lib,
  stdenv,
  fetchurl,
  autoPatchelfHook,
  installShellFiles,
}:

stdenv.mkDerivation (finalAttrs: {
  pname = "barretenberg";
  # `bb`'s version is tracked separately from `noir`/`nargo`'s version.
  # Look up the version compatible with your noir/nargo pin here:
  #   https://raw.githubusercontent.com/AztecProtocol/aztec-packages/next/barretenberg/bbup/bb-versions.json
  # (keys are noir versions, values are the matching bb version to put below)
  version = "5.1.0";

  arch =
    {
      x86_64-linux = "amd64-linux";
      aarch64-linux = "arm64-linux";
      x86_64-darwin = "amd64-darwin";
      aarch64-darwin = "arm64-darwin";
    }
    .${stdenv.hostPlatform.system}
      or (throw "unsupported system for barretenberg: ${stdenv.hostPlatform.system}");

  src = fetchurl {
    url = "https://github.com/AztecProtocol/aztec-packages/releases/download/v${finalAttrs.version}/barretenberg-${finalAttrs.arch}.tar.gz";
    hash = "sha256-AW+hZaXBuGA4a5r6bvSb8JvNFCRjFdfAXN7d2INJ7a8=";
  };

  nativeBuildInputs = [
    installShellFiles
  ]
  ++ lib.optionals stdenv.isLinux [
    autoPatchelfHook
  ];

  # tarball unpacks flat: ./bb (+ possibly SRS/support files)
  sourceRoot = ".";

  installPhase = ''
    runHook preInstall

    install -Dm755 bb $out/bin/bb

    runHook postInstall
  '';

  doInstallCheck = true;
  installCheckPhase = ''
    runHook preInstallCheck
    $out/bin/bb --version
    runHook postInstallCheck
  '';

  meta = {
    description = "Optimized elliptic curve library for the bn128 curve, and a PLONK/Honk SNARK prover (bb)";
    homepage = "https://github.com/AztecProtocol/aztec-packages/tree/master/barretenberg";
    license = with lib.licenses; [
      mit
      asl20
    ];
    platforms = [
      "x86_64-linux"
      "aarch64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ];
    mainProgram = "bb";
    sourceProvenance = with lib.sourceTypes; [ binaryNativeCode ];
  };
})
