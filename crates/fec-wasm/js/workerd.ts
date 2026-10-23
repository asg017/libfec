// The entry under the "workerd" export condition (wrangler, workerd):
// wrangler bundles a `.wasm` import as a compiled WebAssembly.Module, which
// initSync instantiates. Workers can't compile wasm from bytes at run time, so
// index.ts's fetch-and-compile can't work there.
import wasm from "../wasm/libfec_bg.wasm";
import { initSync } from "./native.js";

initSync(wasm);

export * from "./api.js";
