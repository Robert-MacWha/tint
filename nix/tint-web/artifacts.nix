# The join-split circuit's proving artifacts. `gen_artifacts` runs a setup with
# a fixed seed, so these match the ones the deployed verifier was generated
# from.
{
  stdenv,
  rustPlatform,
  rustToolchain,
  tintVendor,
}:

stdenv.mkDerivation {
  pname = "tint-artifacts";
  version = "0.1.0";
  src = ../../packages/crates;

  cargoDeps = tintVendor { };
  nativeBuildInputs = [
    rustToolchain
    rustPlatform.cargoSetupHook
  ];

  buildPhase = ''
    runHook preBuild
    cd tint
    cargo run --offline --release --no-default-features --bin gen_artifacts
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    mkdir -p $out
    cp artifacts/{proving_key,matrices,verifying_key}.bin.br $out/
    runHook postInstall
  '';
}
