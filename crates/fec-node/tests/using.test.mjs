// `using` is a SyntaxError on Node 22, so scripts/test.mjs leaves this file
// out there; it runs on Node 24+, Deno and Bun.
import { test } from "node:test";
import assert from "node:assert/strict";
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
