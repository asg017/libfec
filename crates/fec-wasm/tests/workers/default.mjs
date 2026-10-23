// The Worker for run.mjs: the plain import, which resolves to dist/workerd.js
// under wrangler's "workerd" condition.
import * as lib from "@alex.garcia/libfec-wasm";
import { respond } from "./respond.mjs";

export default { fetch: (request) => respond(lib, request) };
