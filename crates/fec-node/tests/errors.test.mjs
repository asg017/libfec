import { test } from "node:test";
import assert from "node:assert/strict";
import { openSync } from "node:fs";
import { FecError, FecParseError, Filing, MissingMappingError, readHeader } from "../dist/index.js";
import { fixture } from "./helpers.mjs";

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
  if (process.platform !== "win32") {
    assert.equal(err.message, "ENOENT: no such file or directory, open 'nope.fec'");
  }
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

test("FecError keeps its code, status and subclass names", () => {
  const e = new FecError("x", { code: "ERR_FILING_CLOSED" });
  assert.equal(e.name, "FecError");
  assert.equal(e.code, "ERR_FILING_CLOSED");
  assert.equal(e.status, undefined);
  assert.equal(new FecError("y").code, undefined);
  for (const [code, status] of [["ERR_FILING_NOT_FOUND", 404], ["ERR_HTTP", 503]]) {
    const http = new FecError("z", { code, status });
    assert.equal(http.code, code);
    assert.equal(http.status, status);
  }
});

test("ERR_FILING_CLOSED from a closed Filing", () => {
  const f = new Filing(fixture("1921705.fec"));
  f.close();
  for (const view of ["itemizations", "itemizationRecords"]) {
    assert.throws(() => f[view](), (e) => e instanceof FecError && e.name === "FecError" && e.code === "ERR_FILING_CLOSED");
  }
});

test("ERR_FILING_NOT_FOUND and ERR_HTTP from fromId()", async () => {
  const real = globalThis.fetch;
  try {
    for (const [status, code] of [[404, "ERR_FILING_NOT_FOUND"], [500, "ERR_HTTP"]]) {
      globalThis.fetch = async () => new Response("", { status });
      await assert.rejects(Filing.fromId(1), (e) => e instanceof FecError && e.code === code && e.status === status);
    }
  } finally {
    globalThis.fetch = real;
  }
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
