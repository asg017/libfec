// `using` is a SyntaxError on Node 22, so this file is not run there
// (see the Makefile). Phase 1 adds the readers this exercises.
import { test } from "node:test";
import assert from "node:assert/strict";

test("using disposes at block exit", () => {
  const log = [];
  {
    using _r = { [Symbol.dispose]: () => log.push("disposed") };
    log.push("body");
  }
  assert.deepEqual(log, ["body", "disposed"]);
});
