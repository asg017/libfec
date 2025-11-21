// Deno example for fec-wasm
// Run with: deno run --allow-read=pkg,1926611.fec example.deno.ts

import init, { header } from "./pkg/fec_wasm.js";

// Auto-loads the WASM file via import.meta.url
await init();

const fecData = await Deno.readFile("./1926611.fec");

const filingHeader = header(fecData);

filingHeader.record_type;

console.log(JSON.stringify(filingHeader, null, 2));
