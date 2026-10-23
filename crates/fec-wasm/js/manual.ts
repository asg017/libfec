/**
 * The API without loading the wasm on import, for runtimes where the
 * package can't find or compile it itself. Call {@link init} or
 * {@link initSync} once before anything else; until then every call throws
 * `ERR_NOT_INITIALIZED`.
 *
 * ```ts
 * import { initSync, Filing } from "@alex.garcia/libfec-wasm/manual";
 * import wasm from "@alex.garcia/libfec-wasm/libfec_bg.wasm"; // a WebAssembly.Module under wrangler
 * initSync(wasm);
 * ```
 *
 * @module
 */
export { init, initSync, type InitInput, type SyncInitInput } from "./native.js";
export * from "./api.js";
