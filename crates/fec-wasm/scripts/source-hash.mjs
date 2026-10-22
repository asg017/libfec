// A hash of every source the browser build's dist/ is compiled from: the
// shared crates/fec-node/js and this package's js/native.ts. build.mjs
// records it in dist/build.json; crates/fec-node/scripts/build-browser.mjs
// refuses to package a dist/ whose hash doesn't match the sources.
import { createHash } from "node:crypto";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const CRATES = fileURLToPath(new URL("../../", import.meta.url));

function walk(dir) {
  return readdirSync(dir)
    .sort()
    .flatMap((f) => {
      const p = join(dir, f);
      return statSync(p).isDirectory() ? walk(p) : [p];
    });
}

export function jsSourceHash() {
  const h = createHash("sha256");
  for (const p of [...walk(join(CRATES, "fec-node", "js")), join(CRATES, "fec-wasm", "js", "native.ts")]) {
    h.update(relative(CRATES, p).replaceAll("\\", "/"));
    h.update("\0");
    h.update(readFileSync(p));
  }
  return h.digest("hex");
}
