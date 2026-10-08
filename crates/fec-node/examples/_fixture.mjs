// The committed fixture an example reads when no path is given.
import { fileURLToPath } from "node:url";

export const PAC = fileURLToPath(new URL("../../fec-py/tests/fixtures/1721696.fec", import.meta.url));
export const FIXTURES = fileURLToPath(new URL("../../fec-py/tests/fixtures/", import.meta.url));

/** The first command-line argument, in Node, Deno and Bun alike. */
/** @param {string} fallback */
export function arg(fallback) {
  /** @type {any} */
  const g = globalThis;
  const args = g.Deno ? g.Deno.args : process.argv.slice(2);
  return args[0] ?? fallback;
}
