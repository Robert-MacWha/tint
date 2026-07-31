{
  lib,
  rustPlatform,
  fetchFromGitHub,
  versionCheckHook,
  nix-update-script,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "noir";
  version = "1.0.0-beta.22";

  src = fetchFromGitHub {
    owner = "noir-lang";
    repo = "noir";
    tag = "v${finalAttrs.version}";
    hash = "sha256-UkwI6/8HU8mTIRtmB38W6syQSy31/mLUuO67Z8z7D+0=";
  };

  cargoHash = "sha256-v+dc9bqLKlylZY3FljNH8EY7TOHtCRY/lAFnIa8rxe0=";

  buildAndTestSubdir = "tooling/nargo_cli";

  __structuredAttrs = true;

  # build.rs shells out to `git rev-parse` to embed version info,
  # but the source archive has no .git directory
  env = {
    GIT_COMMIT = "v${finalAttrs.version}";
    GIT_DIRTY = "false";
  };

  # Only run unit tests; integration tests try to fetch the Noir stdlib
  # from git, which fails in the Nix sandbox.
  cargoTestFlags = [ "--lib" ];

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
