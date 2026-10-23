// The "./manual" entry: nothing loads the wasm on import, init()/initSync()
// do. In a child process, so no other test file has loaded the wasm first
// (the module state is per process, and shared with the default entry).
import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { pyFixture } from "./helpers.mjs";

const ROOT = fileURLToPath(new URL("..", import.meta.url));
const skip = typeof Deno !== "undefined" || typeof Bun !== "undefined" ? "spawns node" : false;

function run(script) {
  const r = spawnSync(process.execPath, ["--input-type=module", "-e", script, pyFixture("1721696.fec")], {
    cwd: ROOT,
    encoding: "utf8",
  });
  assert.equal(r.status, 0, r.stderr);
  return JSON.parse(r.stdout.trim());
}

const PRELUDE = `
import { readFileSync } from "node:fs";
import * as lib from "@alex.garcia/libfec-wasm/manual";
const fixture = new Uint8Array(readFileSync(process.argv[1]));
const wasmBytes = readFileSync(new URL(import.meta.resolve("@alex.garcia/libfec-wasm/libfec_bg.wasm")));
const codeOf = (f) => { try { f(); } catch (e) { return e.code; } };
`;

test("manual: every call throws ERR_NOT_INITIALIZED until initSync", { skip }, () => {
  const out = run(`${PRELUDE}
const before = {
  filing: codeOf(() => new lib.Filing(fixture)),
  readHeader: codeOf(() => lib.readHeader(fixture)),
  coverEnd: codeOf(() => lib.coverEnd(fixture)),
  columns: codeOf(() => lib.columns("SA11AI", "8.4")),
  version: lib.version,
};
lib.initSync(new WebAssembly.Module(wasmBytes));
lib.initSync(wasmBytes); // a no-op the second time
await lib.init();
console.log(JSON.stringify({ before, formType: new lib.Filing(fixture).formType }));
`);
  assert.deepEqual(out.before, {
    filing: "ERR_NOT_INITIALIZED",
    readHeader: "ERR_NOT_INITIALIZED",
    coverEnd: "ERR_NOT_INITIALIZED",
    columns: "ERR_NOT_INITIALIZED",
    version: out.before.version,
  });
  assert.match(out.before.version, /^\d+\.\d+\.\d+/);
  assert.equal(out.formType, "F3XN");
});

test("manual: init() with no argument loads the packaged wasm", { skip }, () => {
  const out = run(`${PRELUDE}
await lib.init();
console.log(JSON.stringify({ formType: new lib.Filing(fixture).formType }));
`);
  assert.equal(out.formType, "F3XN");
});

test("manual: the same exports as the default entry, plus init and initSync", { skip }, () => {
  const out = run(`
import * as main from "@alex.garcia/libfec-wasm";
import * as manual from "@alex.garcia/libfec-wasm/manual";
console.log(JSON.stringify({ main: Object.keys(main).sort(), manual: Object.keys(manual).sort() }));
`);
  assert.deepEqual(
    out.manual,
    [...out.main, "init", "initSync"].sort(),
  );
});
