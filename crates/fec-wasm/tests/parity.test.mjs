// The browser build matches the napi build on every committed fixture: the
// header, the filer attributes, the typed cover and the cover record, both
// views of the records (both date modes, odd batch sizes), the lazy
// Record.itemization and how a pass ends (a MissingMappingError, at the same
// row). Both builds share js/, so this checks the wasm native layer
// (crates/fec-wasm) against the napi one (crates/fec-node/src) through the
// public API.
import { test } from "node:test";
import assert from "node:assert/strict";
import { FEED, FIXTURES, bytes, napi, outcome, wasm } from "./helpers.mjs";

/** Everything the public API says about one filing up front, as plain data. */
function describe(lib, data, options) {
  const f = new lib.Filing(data, { id: "x", ...options });
  try {
    return {
      id: f.id,
      fecVersion: f.fecVersion,
      header: f.header,
      sourceLength: f.sourceLength,
      filer: [f.formType, f.filerId, f.filerName, f.reportCode, f.coverageFromDate, f.coverageThroughDate],
      cover: f.cover instanceof lib.Unknown ? { unknown: f.cover.toJSON() } : f.cover,
      coverRecord: { ...f.coverRecord.toJSON(), fields: f.coverRecord.fields },
    };
  } finally {
    f.close();
  }
}

/** A record as plain data, fields included. */
const plainRecord = (r) => ({ ...r.toJSON(), fields: r.fields });

/**
 * One full pass as plain data: what it yields, and how it ends. A pass ends
 * at the first row with no column mapping, with that error; the error is
 * part of the data, so both builds must throw it at the same row.
 */
function pass(lib, data, view, options) {
  const f = new lib.Filing(data, options);
  try {
    const items = [];
    let end = "done";
    try {
      for (const x of f[view]()) {
        items.push(
          view === "itemizationRecords"
            ? { ...plainRecord(x), itemization: x.itemization }
            : x instanceof lib.Unknown
              ? { unknown: x.toJSON(), record: plainRecord(x.record) }
              : x,
        );
      }
    } catch (e) {
      if (!(e instanceof lib.MissingMappingError)) throw e;
      end = { name: e.name, rowType: e.rowType, line: e.line, message: e.message };
    }
    return { items, end };
  } finally {
    f.close();
  }
}

assert.ok(FIXTURES.length > 100, `found ${FIXTURES.length} fixtures`);

/** How many fixtures open, for the sanity check at the end. */
let opened = 0;

for (const [label, path] of FIXTURES) {
  test(`parity: ${label}`, () => {
    const data = bytes(path);
    const same = (what, f) => {
      const w = outcome(() => f(wasm));
      const n = outcome(() => f(napi));
      // Two identical TypeErrors would be a broken test (an API that's gone),
      // not parity.
      assert.notEqual(n.error?.name, "TypeError", `${label}: ${what}: ${n.error?.message}`);
      assert.deepEqual(w, n, `${label}: ${what}`);
      return n;
    };
    same("readHeader", (lib) => lib.readHeader(data));
    const d = same("Filing", (lib) => describe(lib, data));
    if (d.error) return; // a fixture the parser rejects: the same error from both is the parity
    opened++;
    same("Filing (dates: date)", (lib) => describe(lib, data, { dates: "date" }));
    same("itemizations", (lib) => pass(lib, data, "itemizations"));
    same("itemizations (batchSize 7)", (lib) => pass(lib, data, "itemizations", { batchSize: 7 }));
    same("itemizationRecords", (lib) => pass(lib, data, "itemizationRecords"));
    same("itemizationRecords (dates: date, batchSize 7)", (lib) => pass(lib, data, "itemizationRecords", { dates: "date", batchSize: 7 }));
  });
}

test("parity: most fixtures open (the loop above compares data, not errors)", () => {
  assert.ok(opened > FIXTURES.length * 0.9, `${opened} of ${FIXTURES.length} opened`);
});

test("parity: columns()", () => {
  for (const [rowType, version] of [["SA11AI", "8.4"], ["SB17", "8.3"], ["F3XN", "8.4"], ["SE", "6.4"], ["ZZ9", "8.4"], ["SA11AI", "0.1"]]) {
    assert.deepEqual(outcome(() => wasm.columns(rowType, version)), outcome(() => napi.columns(rowType, version)), `${rowType} ${version}`);
  }
});

test("parity: bad bytes", () => {
  for (const data of [new Uint8Array(0), new TextEncoder().encode("garbage"), new Uint8Array([0xff, 0xfe, 0x00, 0x1c])]) {
    const n = outcome(() => describe(napi, data));
    assert.equal(n.error?.name, "FecParseError");
    assert.deepEqual(outcome(() => describe(wasm, data)), n);
    assert.deepEqual(outcome(() => wasm.readHeader(data)), outcome(() => napi.readHeader(data)));
  }
});

test("parity: coverEnd", () => {
  for (const [label, path] of FIXTURES) {
    const data = bytes(path);
    const n = napi.coverEnd(data);
    assert.equal(wasm.coverEnd(data), n, label);
    // Around the settled length, where the answer flips.
    for (const k of n === null ? [0, data.length >> 1] : [n - 1, n, n + 1]) {
      const cut = data.subarray(0, Math.max(0, k));
      assert.equal(wasm.coverEnd(cut), napi.coverEnd(cut), `${label} at ${k}`);
    }
  }
});

test("parity: rss", () => {
  assert.deepEqual(wasm.rss.parseFeed(FEED), napi.rss.parseFeed(FEED));
  assert.deepEqual(wasm.rss.parseFeed(FEED.toString("utf8")), napi.rss.parseFeed(FEED.toString("utf8")));
  assert.equal(wasm.rss.feedUrl({ preset: "pac", states: ["CA", "NY"] }), napi.rss.feedUrl({ preset: "pac", states: ["CA", "NY"] }));
  for (const bad of ["<rss><channel><title>x</ti", new Uint8Array([0xff])]) {
    assert.deepEqual(outcome(() => wasm.rss.parseFeed(bad)), outcome(() => napi.rss.parseFeed(bad)));
  }
});

test("parity: the same exports", () => {
  assert.deepEqual(Object.keys(wasm).sort(), Object.keys(napi).sort());
  assert.deepEqual(Object.keys(wasm.rss).sort(), Object.keys(napi.rss).sort());
  assert.equal(wasm.version, napi.version);
});
