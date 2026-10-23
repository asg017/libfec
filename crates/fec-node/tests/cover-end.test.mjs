import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { coverEnd, Filing } from "../dist/index.js";
import { fixture } from "./helpers.mjs";

/** The header, cover and cover record, comparably. */
const opened = (bytes) => {
  const f = new Filing(bytes);
  return JSON.stringify([f.header, f.cover, f.coverRecord, f.formType, f.filerId, f.filerName]);
};

for (const [name, form] of [
  ["1721696.fec", "F3XN"],
  ["1913493.fec", "F99"],
]) {
  test(`coverEnd: ${form}, shortest settled prefix`, () => {
    const full = new Uint8Array(readFileSync(fixture(name)));
    const n = coverEnd(full);
    assert.ok(n !== null && n > 0 && n < full.length, `${n}`);
    assert.equal(coverEnd(full.subarray(0, n - 1)), null);
    assert.equal(coverEnd(full.subarray(0, n)), n);
    assert.equal(coverEnd(full.buffer.slice(full.byteOffset, full.byteOffset + n)), n);
    assert.equal(opened(full.subarray(0, n)), opened(full));
  });
}

test("coverEnd: an F99's text block is part of its cover", () => {
  const full = new Uint8Array(readFileSync(fixture("1913493.fec")));
  const text = new TextDecoder("latin1").decode(full);
  const endText = text.indexOf("[ENDTEXT]");
  assert.equal(coverEnd(full.subarray(0, endText)), null);
  assert.ok(coverEnd(full) > endText);
  assert.match(new Filing(full.subarray(0, coverEnd(full))).cover.text, /Termination of Candidacy/);
});

test("coverEnd: not enough, or settled by an error", () => {
  assert.equal(coverEnd(new Uint8Array(0)), null);
  assert.equal(coverEnd(new TextEncoder().encode("HDR\x1cFEC\x1c8.4\x1cX\x1c1\n")), null);
  const junk = new TextEncoder().encode("garbage\nmore\n");
  const n = coverEnd(junk);
  assert.ok(n !== null);
  assert.throws(() => new Filing(junk.subarray(0, n)), { name: "FecParseError" });
});

test("coverEnd: only bytes", () => {
  for (const bad of ["1721696.fec", null, [1, 2]]) {
    assert.throws(() => coverEnd(bad), TypeError);
  }
});
