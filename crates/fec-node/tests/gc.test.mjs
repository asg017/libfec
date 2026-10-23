// A pass started with next() and then dropped must not hold its native reader
// (an open file descriptor, for a path) until Filing.close(): once the
// generator is garbage-collected, the reader is closed. Needs a GC hook:
// scripts/test.mjs runs node with --expose-gc and deno with
// --v8-flags=--expose-gc; bun has Bun.gc().
//
// Two measures: NativeReader.close() calls, counted by wrapping the method
// (every runtime), and open descriptors in /dev/fd (where it can be listed:
// not on Windows, and Deno wants --allow-all for it).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync } from "node:fs";
import { Filing } from "../dist/index.js";
import { NativeReader } from "../native/native.js";
import { fixture } from "./helpers.mjs";

const PAC = fixture("1721696.fec");

const collect = globalThis.gc ?? (globalThis.Bun ? () => globalThis.Bun.gc(true) : undefined);
const skip = collect === undefined ? "no GC hook (run through scripts/test.mjs)" : false;

/** Open descriptors, or `null` where /dev/fd can't be listed. */
function openFds() {
  try {
    return readdirSync("/dev/fd").length;
  } catch {
    return null;
  }
}

let closes = 0;
const close = NativeReader.prototype.close;
NativeReader.prototype.close = function () {
  closes++;
  return close.call(this);
};

/** GC and yield to the event loop (finalizers run as tasks) until `done()`. */
async function collectUntil(done) {
  for (let i = 0; i < 100 && !done(); i++) {
    collect();
    await new Promise((r) => setTimeout(r, 1));
  }
}

test("an abandoned pass closes its native reader once collected, before close()", { skip }, async () => {
  for (const view of ["itemizations", "itemizationRecords"]) {
    const f = new Filing(PAC, { batchSize: 1 });
    const fds = openFds();
    const start = closes;
    for (let i = 0; i < 50; i++) assert.equal(f[view]().next().done, false);
    assert.equal(closes - start, 0, `${view}: nothing closed while the passes may still be reachable`);
    if (fds !== null) assert.ok(openFds() - fds >= 50, `${view}: 50 started passes hold 50 descriptors`);
    await collectUntil(() => closes - start >= 50);
    assert.equal(closes - start, 50, `${view}: every abandoned reader closed after GC`);
    if (fds !== null) assert.equal(openFds() - fds, 0, `${view}: descriptors released`);
    // The filing still works, and close() afterwards is fine.
    assert.equal([...f[view]()].length, 1387);
    f.close();
  }
});

test("passes still close deterministically: exhausted, return(), throw(), close()", { skip }, () => {
  const f = new Filing(PAC, { batchSize: 1 });
  const fds = openFds();
  const released = (label) => fds !== null && assert.equal(openFds(), fds, label);
  let start = closes;
  for (const _ of f.itemizations());
  assert.equal(closes - start, 1, "exhausted");
  released("exhausted");
  for (const _ of f.itemizationRecords()) break;
  assert.equal(closes - start, 2, "break / return()");
  released("break / return()");
  const g = f.itemizations();
  g.next();
  assert.throws(() => g.throw(new Error("stop")), { message: "stop" });
  assert.equal(closes - start, 3, "throw()");
  released("throw()");
  // Live passes, kept reachable: close() stops and releases every one.
  const live = [f.itemizations(), f.itemizationRecords(), f.itemizations()];
  for (const p of live) p.next();
  if (fds !== null) assert.equal(openFds() - fds, 3);
  start = closes;
  f.close();
  assert.ok(closes - start >= 3, "close()");
  released("close()");
  for (const p of live) assert.throws(() => p.next(), { code: "ERR_FILING_CLOSED" });
});
