// Typed itemizations as newline-delimited JSON on stdout (Node, Deno and Bun).
import { Filing } from "../dist/index.js";
import { PAC, arg } from "./_fixture.mjs";

/** @type {any} */
const g = globalThis;
/** @type {(s: string) => unknown} */
const write = g.Deno
  ? ((w) => (/** @type {string} */ s) => w.write(new TextEncoder().encode(s)))(g.Deno.stdout.writable.getWriter())
  : (s) => process.stdout.write(s);

const filing = new Filing(arg(PAC));
try {
  // An Unknown serializes as { type: "Unknown", formType, values }.
  for (const item of filing.itemizations().take(2)) {
    await write(JSON.stringify(item) + "\n");
  }
} finally {
  filing.close();
}
