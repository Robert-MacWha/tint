{
  lib,
  rustPlatform,
  fetchFromGitHub,
  versionCheckHook,
  nix-update-script,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "noir";
  version = "0.39.0";

  src = fetchFromGitHub {
    owner = "noir-lang";
    repo = "noir";
    tag = "v${finalAttrs.version}";
    hash = "sha256-SsF96UUlvVsv6uQN7bsCbvkGgcdYPdRD6AQ6ajzODpA=";
  };

  cargoHash = "sha256-FRv5OjyyecyyDdCsZrYNv7wjSSn8leADDOzzXzKx0/0=";

  # Build both nargo and noir-inspector explicitly
  cargoBuildFlags = [
    "--package"
    "nargo_cli"
    "--package"
    "noir_inspector"
  ];

  # cargoTestFlags = [
  #   "--package"
  #   "nargo_cli"
  #   "--lib"
  # ];

  doCheck = false;

  preCheck = ''
    export HOME=$TMPDIR
  '';

  __structuredAttrs = true;

  env = {
    GIT_COMMIT = "v${finalAttrs.version}";
    GIT_DIRTY = "false";
    HOME = "/tmp";
  };

  nativeInstallCheckInputs = [ versionCheckHook ];
  doInstallCheck = true;

  passthru = {
    updateScript = nix-update-script { };
  };

  meta = with lib; {
    description = "Domain specific language for writing zero-knowledge proofs";
    homepage = "https://noir-lang.org";
    changelog = "https://github.com/noir-lang/noir/releases/tag/v${finalAttrs.version}";
    license = with licenses; [
      mit
      asl20
    ];
    mainProgram = "nargo";
    maintainers = with maintainers; [ dhkl ];
  };
})
