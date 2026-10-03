import { test } from "node:test";
import assert from "node:assert/strict";
import { openSync } from "node:fs";
import { FecError, FecParseError, MissingMappingError, readHeader } from "../dist/index.js";

test("a missing path is a Node-style ENOENT, not a FecError", () => {
  let err;
  try {
    readHeader("nope.fec");
  } catch (e) {
    err = e;
  }
  assert.ok(err instanceof Error);
  assert.ok(!(err instanceof FecError));
  assert.equal(err.code, "ENOENT");
  assert.equal(err.path, "nope.fec");
  assert.equal(err.syscall, "open");
  assert.equal(err.message, "ENOENT: no such file or directory, open 'nope.fec'");
  // The same message and errno Node's own fs gives.
  try {
    openSync("nope.fec");
  } catch (e) {
    assert.equal(err.message, e.message);
    if (process.platform !== "win32") assert.equal(err.errno, e.errno);
  }
});

test("unparseable bytes: FecParseError < FecError < Error", () => {
  let err;
  try {
    readHeader(new TextEncoder().encode("garbage"));
  } catch (e) {
    err = e;
  }
  assert.ok(err instanceof FecParseError);
  assert.ok(err instanceof FecError);
  assert.ok(err instanceof Error);
  assert.equal(err.code, "FEC_PARSE");
  assert.equal(err.name, "FecParseError");
});

test("MissingMappingError carries its context", () => {
  const e = new MissingMappingError("ZZ9", "8.4", 3);
  assert.equal(e.name, "MissingMappingError");
  assert.equal(e.code, "FEC_MISSING_MAPPING");
  assert.equal(e.rowType, "ZZ9");
  assert.equal(e.fecVersion, "8.4");
  assert.equal(e.line, 3);
  assert.match(e.message, /ZZ9.*8\.4.*line 3/);
  assert.ok(e instanceof FecError);
});

test("FecError keeps its code and subclass names", () => {
  const e = new FecError("x", { code: "ERR_FILING_CLOSED" });
  assert.equal(e.name, "FecError");
  assert.equal(e.code, "ERR_FILING_CLOSED");
  assert.equal(new FecError("y").code, undefined);
});

test("fromNative maps on `code` alone, whatever threw it", async () => {
  const { fromNative } = await import("../dist/errors.js");
  const parse = fromNative({ code: "FEC_PARSE", message: "bad cover" });
  assert.ok(parse instanceof FecParseError);
  assert.equal(parse.message, "bad cover");
  const missing = fromNative({ code: "ENOENT", message: "x" }, { path: "a.fec" });
  assert.equal(missing.code, "ENOENT");
  assert.equal(missing.path, "a.fec");
  assert.equal(fromNative({ code: "ERR_FILING_CLOSED" }).code, "ERR_FILING_CLOSED");
  const other = { code: "SOMETHING_ELSE" };
  assert.equal(fromNative(other), other);
});
