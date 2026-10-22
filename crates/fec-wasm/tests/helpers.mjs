import { readdirSync, readFileSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { join } from "node:path";

/** The browser build (this package). */
export const wasm = await import("../dist/index.js");

/**
 * The napi build (crates/fec-node, `make build` there), the reference the
 * browser build must match value for value.
 */
export const napi = await import("../../fec-node/dist/index.js");

const CRATES = fileURLToPath(new URL("../../", import.meta.url));

function walk(dir) {
  return readdirSync(dir).flatMap((f) => {
    const p = join(dir, f);
    return statSync(p).isDirectory() ? walk(p) : p.endsWith(".fec") ? [p] : [];
  });
}

/** Every committed .fec fixture in the repo: [label, path]. */
export const FIXTURES = [
  "fec-py/tests/fixtures",
  "fec-parser/tests/fixtures",
  "fec-cli/tests/fixtures",
  "fec-node/tests/fixtures",
]
  .flatMap((d) => walk(join(CRATES, d)))
  .sort()
  .map((p) => [p.slice(CRATES.length), p]);

/** A fixture's bytes, as a fresh Uint8Array. */
export function bytes(path) {
  return new Uint8Array(readFileSync(path));
}

/** fec-py's fixture by name, e.g. "1721696.fec". */
export function pyFixture(name) {
  return join(CRATES, "fec-py", "tests", "fixtures", name);
}

/** fec-rss's saved feed. */
export const FEED = readFileSync(join(CRATES, "fec-rss", "tests", "fixtures", "all.xml"));

/** What `f()` returned or threw, comparably: `{ value }` or `{ error: {name, code, message} }`. */
export function outcome(f) {
  try {
    return { value: f() };
  } catch (e) {
    return { error: { name: e?.constructor?.name, code: e?.code, message: e?.message } };
  }
}
