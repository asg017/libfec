// The default entry: loads the wasm (top-level await) from next to the
// package, then everything is the shared API (crates/fec-node/js/index.ts,
// which scripts/build.mjs compiles as api.ts).
import { init } from "./native.js";

await init();

export * from "./api.js";
