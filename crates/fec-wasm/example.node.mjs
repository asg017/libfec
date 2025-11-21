// Node.js example for fec-wasm
// Run with: node example.node.mjs

import { readFile } from "fs/promises";
import init, { header } from "./pkg/fec_wasm.js";

// Initialize the WASM module by loading the .wasm file
const wasmBuffer = await readFile("./pkg/fec_wasm_bg.wasm");
await init({ module_or_path: wasmBuffer });

// Read the FEC file
const fecData = await readFile("./1926611.fec");

// Parse the header
const filingHeader = header(fecData);

console.log(JSON.stringify(filingHeader, null, 2));