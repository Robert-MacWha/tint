// Bootstraps the tint worker (`tint-web-worker`), built into ./worker by `just web`.

// Imported by URL so dx doesn't bundle the glue into this script: the rayon
// thread pool re-imports the glue by its own URL in each of its workers.
const glue = new URL("./worker/tint_web_worker.js", import.meta.url);
const { default: init } = await import(glue.href);
await init({ module_or_path: new URL("./worker/tint_web_worker_bg.wasm", import.meta.url) });
