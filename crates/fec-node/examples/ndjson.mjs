// Typed rows as newline-delimited JSON on stdout (Node, Deno and Bun).
import { open } from "../dist/index.js";
import { PAC, arg } from "./_fixture.mjs";

/** @type {any} */
const g = globalThis;
/** @type {(s: string) => unknown} */
const write = g.Deno
  ? ((w) => (/** @type {string} */ s) => w.write(new TextEncoder().encode(s)))(g.Deno.stdout.writable.getWriter())
  : (s) => process.stdout.write(s);

const filing = open(arg(PAC));
let n = 0;
try {
  for (const row of filing.itemizations()) {
    if (n++ === 2) break; // the first two, to keep the output short
    await write(JSON.stringify(row) + "\n");
  }
} finally {
  filing.close();
}
