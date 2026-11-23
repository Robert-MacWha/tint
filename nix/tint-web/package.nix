# tint-web as static files, to serve alongside a /config.json describing the
# deployment, as ./module.nix does. Mirrors `just web-worker`.
{
  stdenv,
  rustPlatform,
  rustToolchain,
  tintVendor,
  tint-artifacts,
  wasm-bindgen-cli_0_2_108,
  binaryen,
  dioxus-cli,
}:

stdenv.mkDerivation {
  pname = "tint-web";
  version = "0.1.0";
  src = ../../packages/crates;

  cargoDeps = tintVendor { withStd = true; };
  nativeBuildInputs = [
    rustToolchain
    rustPlatform.cargoSetupHook
    wasm-bindgen-cli_0_2_108
    binaryen
    dioxus-cli
  ];

  buildPhase = ''
    runHook preBuild
    export HOME=$TMPDIR

    # See `just web-worker` for why the worker needs these flags.
    RUSTC_BOOTSTRAP=1 \
    RUSTFLAGS="-C target-feature=+atomics,+bulk-memory \
        -C link-arg=--shared-memory -C link-arg=--import-memory \
        -C link-arg=--max-memory=4294967296 \
        -C link-arg=--export=__wasm_init_tls -C link-arg=--export=__tls_size \
        -C link-arg=--export=__tls_align -C link-arg=--export=__tls_base \
        -C link-arg=--export=__heap_base" \
        cargo build --offline --release -p tint-web-worker \
        --target wasm32-unknown-unknown -Z build-std=panic_abort,std
    wasm-bindgen --target web --out-dir tint-web/public/worker \
        target/wasm32-unknown-unknown/release/tint_web_worker.wasm
    wasm-opt -O3 --enable-threads --enable-bulk-memory --enable-mutable-globals \
        --enable-nontrapping-float-to-int --enable-sign-ext --enable-reference-types \
        --enable-multivalue tint-web/public/worker/tint_web_worker_bg.wasm \
        -o tint-web/public/worker/tint_web_worker_bg.wasm

    mkdir -p tint-web/public/artifacts
    for f in proving_key matrices verifying_key; do
        cp ${tint-artifacts}/$f.bin.br tint-web/public/artifacts/$f.bin
    done

    cd tint-web
    dx bundle --web --release --debug-symbols false --out-dir dist
    find dist/public -type f \( -name '*.bin' -o -name '*.wasm' -o -name '*.js' \) \
        -exec gzip -k -9 {} +
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    cp -r dist/public $out
    runHook postInstall
  '';
}
