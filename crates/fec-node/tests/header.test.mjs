import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

import { readHeader, version } from "../dist/index.js";
import { fixture, LEGACY_FIXTURES } from "./helpers.mjs";

test("version is the workspace version", () => {
  assert.match(version, /^\d+\.\d+\.\d+/);
});

test("readHeader: 8.5 F3N from a path", () => {
  const h = readHeader(fixture("1921705.fec"));
  assert.equal(h.recordType, "HDR");
  assert.equal(h.efType, "FEC");
  assert.equal(h.fecVersion, "8.5");
  assert.equal(h.softwareName, "FECfile");
  assert.equal(h.softwareVersion, "8.5.0.0(f33)");
  assert.equal(h.reportId, null);
  assert.equal(h.reportNumber, null);
  assert.equal(h.comment, null);
  assert.equal(h.style, "hdr");
  assert.equal(h.delimiter, "fs");
  assert.equal(h.isPaper, false);
  assert.deepEqual(h.legacyFields, {});
  assert.ok(Object.isFrozen(h));
});

test("readHeader: 8.4 FECFile", () => {
  const h = readHeader(fixture("1721696.fec"));
  assert.equal(h.fecVersion, "8.4");
  assert.equal(h.softwareName, "FECFile");
  assert.equal(h.reportNumber, "0");
});

test("readHeader: report id and number", () => {
  const h = readHeader(fixture("1913562.fec"));
  assert.equal(h.reportId, "FEC-1228863");
  assert.equal(h.reportNumber, "22");
});

test("readHeader: Uint8Array, Buffer, ArrayBuffer and file: URL match the path", () => {
  const path = fixture("1921705.fec");
  const expected = readHeader(path);
  const buf = readFileSync(path);
  assert.deepEqual(readHeader(buf), expected);
  assert.deepEqual(readHeader(new Uint8Array(buf)), expected);
  const ab = buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength);
  assert.deepEqual(readHeader(ab), expected);
  assert.deepEqual(readHeader(pathToFileURL(path)), expected);
});

test("readHeader: reads only the header line", () => {
  const bytes = new TextEncoder().encode(
    "HDR\x1cFEC\x1c8.4\x1cX\x1c1.0\x1c\x1c\n\x00\xff garbage, not a cover\n",
  );
  assert.equal(readHeader(bytes).fecVersion, "8.4");
});

test("readHeader: 1.x /* Header block", () => {
  const h = readHeader(fixture("1.02_497.fec", LEGACY_FIXTURES));
  assert.equal(h.recordType, "/*");
  assert.equal(h.fecVersion, "1.02");
  assert.equal(h.style, "legacy_block");
  assert.equal(h.legacyFields.Form_Name, "F3N");
  assert.equal(h.scheduleCounts.SA11AI, "00147");
  assert.ok(Object.isFrozen(h.legacyFields));
});

test("readHeader: paper filing", () => {
  const h = readHeader(fixture("P3.4_1215766.fec", LEGACY_FIXTURES));
  assert.equal(h.fecVersion, "P3.4");
  assert.equal(h.style, "paper");
  assert.equal(h.isPaper, true);
});

test("readHeader: bad source types are TypeErrors", () => {
  assert.throws(() => readHeader(123), TypeError);
  assert.throws(() => readHeader({}), TypeError);
  assert.throws(() => readHeader(null), TypeError);
  assert.throws(() => readHeader(new URL("https://example.com/1.fec")), TypeError);
});

test("readHeader: a missing path throws", () => {
  assert.throws(() => readHeader("missing.fec"), /NotFound/);
});

test("readHeader: a file with no HDR throws", () => {
  assert.throws(() => readHeader(new TextEncoder().encode("SA11AI\x1cC001\n")));
});
