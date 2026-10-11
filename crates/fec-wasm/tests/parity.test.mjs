// The browser build matches the napi build on every committed fixture: the
// header, the typed cover, every typed itemization, every row's values (both
// date modes), the raw records, invalid values, skipped rows and errors.
// Both builds share js/, so this checks the wasm native layer (crates/fec-wasm)
// against the napi one (crates/fec-node/src) through the public API.
import { test } from "node:test";
import assert from "node:assert/strict";
import { FEED, FIXTURES, bytes, napi, outcome, wasm } from "./helpers.mjs";

/** Everything the public API says about one filing, as plain data. */
function describe(lib, data, options) {
  const f = lib.open(data, { id: "x", ...options });
  try {
    return {
      id: f.id,
      fecVersion: f.fecVersion,
      header: f.header,
      cover: f.cover,
      coverSummary: f.coverSummary,
      coverRow: f.coverRow.toJSON(),
    };
  } finally {
    f.close();
  }
}

function read(lib, data, mode, options) {
  const f = lib.open(data, options);
  try {
    const rows = [];
    for (const r of f[mode]()) rows.push(mode === "records" ? r : r.toJSON());
    return {
      rows,
      invalidValues: f.invalidValues,
      invalidValueCount: f.invalidValueCount,
      skippedRows: f.skippedRows,
    };
  } finally {
    f.close();
  }
}

assert.ok(FIXTURES.length > 100, `found ${FIXTURES.length} fixtures`);

for (const [label, path] of FIXTURES) {
  test(`parity: ${label}`, () => {
    const data = bytes(path);
    const same = (what, f) => assert.deepEqual(outcome(() => f(wasm)), outcome(() => f(napi)), `${label}: ${what}`);
    same("readHeader", (lib) => lib.readHeader(data));
    same("open", (lib) => describe(lib, data));
    same("open (dates: date)", (lib) => describe(lib, data, { dates: "date" }));
    same("itemizations", (lib) => read(lib, data, "itemizations"));
    same("rows (skip unknown)", (lib) => read(lib, data, "rows", { unknownRows: "skip" }));
    same("rows (throw unknown)", (lib) => read(lib, data, "rows"));
    same("rows (dates: date, batchSize 7)", (lib) => read(lib, data, "rows", { dates: "date", batchSize: 7, unknownRows: "skip" }));
    same("records", (lib) => read(lib, data, "records"));
    same("read()", (lib) => {
      const filing = lib.read(data, { unknownRows: "skip" });
      return { rows: filing.rows.map((r) => r.toJSON()), itemizations: filing.itemizations().map((r) => r.toJSON()) };
    });
  });
}

test("parity: columns()", () => {
  for (const [rowType, version] of [["SA11AI", "8.4"], ["SB17", "8.3"], ["F3XN", "8.4"], ["SE", "6.4"], ["ZZ9", "8.4"], ["SA11AI", "0.1"]]) {
    assert.deepEqual(outcome(() => wasm.columns(rowType, version)), outcome(() => napi.columns(rowType, version)), `${rowType} ${version}`);
  }
});

test("parity: bad bytes", () => {
  for (const data of [new Uint8Array(0), new TextEncoder().encode("garbage"), new Uint8Array([0xff, 0xfe, 0x00, 0x1c])]) {
    assert.deepEqual(outcome(() => describe(wasm, data)), outcome(() => describe(napi, data)));
    assert.deepEqual(outcome(() => wasm.readHeader(data)), outcome(() => napi.readHeader(data)));
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
