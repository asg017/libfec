// Packaging: @alex.garcia/libfec resolves to this build under the "browser"
// export condition (and at @alex.garcia/libfec/browser), and to napi otherwise.
// Runs crates/fec-node/scripts/build-browser.mjs first, which copies this
// package's dist/ and wasm/ into crates/fec-node/browser/. With --allow-dev,
// so this runs against `make build-dev` too: it checks how the package
// resolves, not how the wasm was optimized (packing refuses a dev build).
import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const NODE_PKG = fileURLToPath(new URL("../../fec-node/", import.meta.url));

// Self-reference: inside crates/fec-node, `@alex.garcia/libfec` resolves through
// its own package.json "exports", as it would for a dependent.
const PROBE = `
import * as main from "@alex.garcia/libfec";
import * as sub from "@alex.garcia/libfec/browser";
const codeOf = (lib) => { try { new lib.Filing("missing.fec"); } catch (e) { return e.code; } };
console.log(JSON.stringify({ code: codeOf(main), subCode: codeOf(sub), exports: Object.keys(main).sort(), subExports: Object.keys(sub).sort() }));
`;

function probe(conditions) {
  const r = spawnSync(process.execPath, [...conditions, "--input-type=module", "-e", PROBE], { cwd: NODE_PKG, encoding: "utf8" });
  assert.equal(r.status, 0, r.stderr);
  return JSON.parse(r.stdout.trim());
}

test("the browser condition picks the wasm build", { skip: typeof Deno !== "undefined" || typeof Bun !== "undefined" ? "spawns node" : false }, () => {
  const copy = spawnSync(process.execPath, ["scripts/build-browser.mjs", "--allow-dev"], { cwd: NODE_PKG, encoding: "utf8" });
  assert.equal(copy.status, 0, copy.stderr);
  const plain = probe([]);
  assert.equal(plain.code, "ENOENT");
  assert.equal(plain.subCode, "ERR_UNSUPPORTED_SOURCE");
  const browser = probe(["--conditions=browser"]);
  assert.equal(browser.code, "ERR_UNSUPPORTED_SOURCE");
  assert.equal(browser.subCode, "ERR_UNSUPPORTED_SOURCE");
  // The same API either way.
  assert.deepEqual(plain.exports, plain.subExports);
  assert.deepEqual(browser.exports, plain.exports);
  for (const name of ["Filing", "Record", "Unknown", "MissingMappingError", "FecError", "columns", "readHeader", "rss"]) {
    assert.ok(plain.exports.includes(name), name);
  }
  for (const gone of ["open", "read", "FilingReader", "Row"]) assert.ok(!plain.exports.includes(gone), gone);
});

test("both packages ship the same browser files", () => {
  const pkg = (p) => JSON.parse(readFileSync(new URL(p, import.meta.url), "utf8"));
  const wasmFiles = pkg("../package.json").files.filter((f) => f.startsWith("dist") || f.startsWith("wasm/"));
  const nodeFiles = pkg("../../fec-node/package.json").files.filter((f) => f.startsWith("browser/")).map((f) => f.slice("browser/".length));
  assert.deepEqual(nodeFiles, wasmFiles);
  assert.equal(pkg("../package.json").version, pkg("../../fec-node/package.json").version);
});

test("both packages ship real copies of the root license files", () => {
  // npm pack skips symlinks, so these must be regular files.
  for (const pkg of ["fec-node", "fec-wasm"]) {
    for (const name of ["LICENSE-MIT", "LICENSE-APACHE"]) {
      const root = readFileSync(new URL(`../../../${name}`, import.meta.url), "utf8");
      assert.equal(readFileSync(new URL(`../../${pkg}/${name}`, import.meta.url), "utf8"), root, `${pkg}/${name}`);
    }
  }
});
