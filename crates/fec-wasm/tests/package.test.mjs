// Packaging: @asg017/libfec resolves to this build under the "browser"
// export condition (and at @asg017/libfec/browser), and to napi otherwise.
// Runs crates/fec-node/scripts/build-browser.mjs first, which copies this
// package's dist/ and wasm/ into crates/fec-node/browser/.
import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const NODE_PKG = fileURLToPath(new URL("../../fec-node/", import.meta.url));

// Self-reference: inside crates/fec-node, `@asg017/libfec` resolves through
// its own package.json "exports", as it would for a dependent.
const PROBE = `
import { open } from "@asg017/libfec";
import * as sub from "@asg017/libfec/browser";
let code; try { open("missing.fec"); } catch (e) { code = e.code; }
let subCode; try { sub.open("missing.fec"); } catch (e) { subCode = e.code; }
console.log(JSON.stringify({ code, subCode }));
`;

function probe(conditions) {
  const r = spawnSync(process.execPath, [...conditions, "--input-type=module", "-e", PROBE], { cwd: NODE_PKG, encoding: "utf8" });
  assert.equal(r.status, 0, r.stderr);
  return JSON.parse(r.stdout.trim());
}

test("the browser condition picks the wasm build", { skip: typeof Deno !== "undefined" || typeof Bun !== "undefined" ? "spawns node" : false }, () => {
  const copy = spawnSync(process.execPath, ["scripts/build-browser.mjs"], { cwd: NODE_PKG, encoding: "utf8" });
  assert.equal(copy.status, 0, copy.stderr);
  assert.deepEqual(probe([]), { code: "ENOENT", subCode: "ERR_UNSUPPORTED_SOURCE" });
  assert.deepEqual(probe(["--conditions=browser"]), { code: "ERR_UNSUPPORTED_SOURCE", subCode: "ERR_UNSUPPORTED_SOURCE" });
});

test("both packages ship the same browser files", () => {
  const pkg = (p) => JSON.parse(readFileSync(new URL(p, import.meta.url), "utf8"));
  const wasmFiles = pkg("../package.json").files.filter((f) => f.startsWith("dist") || f.startsWith("wasm/"));
  const nodeFiles = pkg("../../fec-node/package.json").files.filter((f) => f.startsWith("browser/")).map((f) => f.slice("browser/".length));
  assert.deepEqual(nodeFiles, wasmFiles);
  assert.equal(pkg("../package.json").version, pkg("../../fec-node/package.json").version);
});
