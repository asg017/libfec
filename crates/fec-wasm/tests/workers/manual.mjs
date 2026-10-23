// The Worker for run.mjs: the "./manual" entry, given the module wrangler
// compiles from the package's `.wasm` export.
import * as lib from "@alex.garcia/libfec-wasm/manual";
import wasm from "@alex.garcia/libfec-wasm/libfec_bg.wasm";
import { respond } from "./respond.mjs";

lib.initSync(wasm);

export default { fetch: (request) => respond(lib, request) };
