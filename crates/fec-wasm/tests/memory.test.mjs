// A filing's bytes are copied into wasm memory once, by new Filing(), and
// every pass reads that copy (crates/fec-wasm/src/reader.rs, Rc<[u8]>): wasm
// memory must not grow by the file's size per pass. The browser harness runs
// the same check (tests/browser/memory.mjs) in Chrome. Its own file, so the
// wasm heap starts small and freed space from other tests can't hide a copy.
import { test } from "node:test";
import assert from "node:assert/strict";
import init from "../wasm/libfec.js";
import { bytes, pyFixture, wasm } from "./helpers.mjs";
import { bigFiling, memoryPerPass, memoryProblems } from "./browser/memory.mjs";

// Already initialized by dist/native.js (the same module instance), so this
// just hands back the instance's exports.
const { memory } = await init();
const PAC = bytes(pyFixture("1721696.fec"));

test("wasm memory doesn't grow by the file size per pass", () => {
  const { data } = bigFiling(PAC, 32);
  const r = memoryPerPass(wasm, memory, data);
  assert.deepEqual(memoryProblems(r), [], JSON.stringify(r));
});

test("the big filing reads in full: every row, in both views", () => {
  const { data, rows } = bigFiling(PAC, 2);
  const f = new wasm.Filing(data);
  for (const view of ["itemizations", "itemizationRecords"]) {
    let n = 0;
    for (const _ of f[view]()) n++;
    assert.equal(n, rows, view);
  }
  f.close();
});
