{
  lib,
  rustPlatform,
  fetchFromGitHub,
  nix-update-script,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "provekit-cli";
  version = "1.0.0";

  src = fetchFromGitHub {
    owner = "worldfnd";
    repo = "provekit";
    tag = "v${finalAttrs.version}";
    hash = "sha256-UuuSVBL6vCU1nlSbI17h7ZYBZXf7+5nGMA4ac/u1hpM=";
  };

  cargoHash = "sha256-bcFnBy5skrBvkfIgooEkjT77Cu2ub+Rb+QSI4agZheQ=";

  buildAndTestSubdir = "tooling/cli";

  __structuredAttrs = true;

  env = {
    GIT_COMMIT = "v${finalAttrs.version}";
    GIT_DIRTY = "false";
  };

  cargoTestFlags = [ "--bins" ];

  doInstallCheck = false;

  passthru = {
    updateScript = nix-update-script { };
  };

  meta = with lib; {
    description = "Client side zero-knowledge proving";
    homepage = "https://github.com/worldfnd/provekit";
    changelog = "https://github.com/worldfnd/provekit/releases/tag/v${finalAttrs.version}";
    license = with licenses; [ mit ];
    mainProgram = "provekit-cli";
  };
})
