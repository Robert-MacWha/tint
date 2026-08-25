{
  lib,
  rustPlatform,
  fetchFromGitHub,
  pkg-config,
  m4,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "picus";
  version = "main";

  src = fetchFromGitHub {
    owner = "chyanju";
    repo = "Picus";
    rev = "${finalAttrs.version}";
    hash = "sha256-wgCuJZDWI3TqxHHAzydG5oA43pui/WcxUWlmTQsCn5I=";
  };

  cargoLock.lockFile = "${finalAttrs.src}/Cargo.lock";

  cargoBuildFlags = [
    "-p"
    "picus-cli"
  ];

  nativeBuildInputs = [
    pkg-config
    m4
  ];

  # crates/picus's r1cs_smoke test needs a circomlib git submodule and
  # circom-compiled circuits we don't have; skip it as the test itself
  # documents.
  PICUS_SKIP_PLDI_SMOKE = "1";

  doInstallCheck = true;
  installCheckPhase = ''
    runHook preInstallCheck
    $out/bin/picus --help >/dev/null
    runHook postInstallCheck
  '';

  meta = {
    description = "Detects under-constrained (non-unique) signals in ZKP circuits (QED^2)";
    homepage = "https://github.com/chyanju/Picus";
    license = lib.licenses.mit;
    maintainers = [ ];
    platforms = lib.platforms.unix;
    mainProgram = "picus";
  };
})
