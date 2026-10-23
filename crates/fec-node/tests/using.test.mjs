// `using` is a SyntaxError on Node 22, so scripts/test.mjs leaves this file
// out there; it runs on Node 24+, Deno and Bun.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { Filing } from "../dist/index.js";
import { fixture } from "./helpers.mjs";

test("using disposes at block exit", () => {
  const log = [];
  {
    using _r = { [Symbol.dispose]: () => log.push("disposed") };
    log.push("body");
  }
  assert.deepEqual(log, ["body", "disposed"]);
});

test("using closes a Filing", () => {
  let filing;
  {
    using f = new Filing(fixture("1921705.fec"));
    filing = f;
    assert.equal([...f.itemizations()].length, 20);
  }
  assert.equal(filing.closed, true);
});

test("using: a pass that outlives the block throws ERR_FILING_CLOSED", () => {
  let pass;
  {
    using f = new Filing(fixture("1721696.fec"), { batchSize: 10 });
    pass = f.itemizationRecords();
    assert.equal(pass.next().done, false);
  }
  assert.throws(() => pass.next(), { code: "ERR_FILING_CLOSED" });
});

test("using: an early break out of a pass, then more passes, then disposal", () => {
  let filing;
  {
    using f = new Filing(fixture("1721696.fec"));
    filing = f;
    for (const item of f.itemizations()) {
      assert.equal(item.type, "ScheduleA");
      break;
    }
    assert.equal([...f.itemizationRecords()].length, 1387);
  }
  assert.equal(filing.closed, true);
  // The up-front data outlives the source.
  assert.equal(filing.cover.type, "Form3X");
  assert.equal(filing.coverRecord.rowType, "F3XN");
});

test("using: a Filing from fromId()", async () => {
  const bytes = readFileSync(fixture("1721696.fec"));
  const real = globalThis.fetch;
  globalThis.fetch = async () => new Response(bytes);
  let filing;
  try {
    using f = await Filing.fromId(1721696);
    filing = f;
    assert.equal([...f.itemizations()].length, 1387);
  } finally {
    globalThis.fetch = real;
  }
  assert.equal(filing.closed, true);
});
