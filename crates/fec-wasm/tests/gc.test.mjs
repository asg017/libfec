// The browser build shares js/filing.ts with the napi build, so an abandoned
// pass (started with next(), then dropped) must close its wasm reader once the
// generator is garbage-collected, not hold its parser memory until
// Filing.close(). wasm-bindgen objects are never freed by GC here (no
// --weak-refs), so this close() is what releases the parser. Counted by
// wrapping the wasm NativeReader's close(). Needs a GC hook: scripts/test.mjs
// runs node with --expose-gc and deno with --v8-flags=--expose-gc; bun has
// Bun.gc(). The file-descriptor side is in crates/fec-node/tests/gc.test.mjs.
import { test } from "node:test";
import assert from "node:assert/strict";
import { NativeReader } from "../wasm/libfec.js";
import { bytes, pyFixture, wasm } from "./helpers.mjs";

const PAC = bytes(pyFixture("1721696.fec"));

const collect = globalThis.gc ?? (globalThis.Bun ? () => globalThis.Bun.gc(true) : undefined);
const skip = collect === undefined ? "no GC hook (run through scripts/test.mjs)" : false;

let closes = 0;
const close = NativeReader.prototype.close;
NativeReader.prototype.close = function () {
  closes++;
  return close.call(this);
};

async function collectUntil(done) {
  for (let i = 0; i < 100 && !done(); i++) {
    collect();
    await new Promise((r) => setTimeout(r, 1));
  }
}

test("an abandoned pass closes its wasm reader once collected, before close()", { skip }, async () => {
  for (const view of ["itemizations", "itemizationRecords"]) {
    const f = new wasm.Filing(PAC, { batchSize: 1 });
    const start = closes;
    for (let i = 0; i < 50; i++) assert.equal(f[view]().next().done, false);
    assert.equal(closes - start, 0, view);
    await collectUntil(() => closes - start >= 50);
    assert.equal(closes - start, 50, `${view}: every abandoned reader closed after GC`);
    assert.equal([...f[view]()].length, 1387);
    f.close();
  }
});

test("close() still closes every live pass", { skip }, () => {
  const f = new wasm.Filing(PAC, { batchSize: 1 });
  const live = [f.itemizations(), f.itemizationRecords()];
  for (const p of live) p.next();
  const start = closes;
  f.close();
  assert.equal(closes - start, 2);
  for (const p of live) assert.throws(() => p.next(), { code: "ERR_FILING_CLOSED" });
});
