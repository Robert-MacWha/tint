// Bootstraps the tint worker (`tint-web-worker`), built into ./worker by `just web`.
import init from "./worker/tint_web_worker.js";

// dx bundles this script with its import, which moves the glue's default wasm
// path to the site root, so point it at the wasm explicitly.
await init({ module_or_path: new URL("./worker/tint_web_worker_bg.wasm", import.meta.url) });
