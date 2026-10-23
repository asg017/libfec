// What only the browser build does: bytes-only sources, and the wasm loaded
// before the module resolves (so the API is sync after import).
import { test } from "node:test";
import assert from "node:assert/strict";
import { pathToFileURL } from "node:url";
import { bytes, pyFixture, wasm } from "./helpers.mjs";

const PAC = pyFixture("1721696.fec");

test("paths and file: URLs are a TypeError with code ERR_UNSUPPORTED_SOURCE", () => {
  for (const source of [PAC, pathToFileURL(PAC)]) {
    for (const f of [() => new wasm.Filing(source), () => wasm.readHeader(source)]) {
      assert.throws(f, (e) => e instanceof TypeError && e.code === "ERR_UNSUPPORTED_SOURCE" && /bytes, not paths/.test(e.message));
    }
  }
  // Still a TypeError for anything else.
  assert.throws(() => new wasm.Filing(42), TypeError);
});

test("Uint8Array, Buffer and ArrayBuffer sources", () => {
  const data = bytes(PAC);
  const count = (src) => {
    const f = new wasm.Filing(src);
    try {
      return [...f.itemizations()].length;
    } finally {
      f.close();
    }
  };
  const n = count(data);
  assert.equal(n, 1387);
  assert.equal(count(data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength)), n);
  assert.equal(count(Buffer.from(data)), n);
  // A view into a larger buffer reads only its own bytes.
  const padded = new Uint8Array(data.length + 10);
  padded.set(data, 5);
  assert.equal(count(padded.subarray(5, 5 + data.length)), n);
});

test("the bytes are copied: mutating the source after new Filing() changes nothing", () => {
  const data = bytes(PAC);
  const f = new wasm.Filing(data, { id: "1721696" });
  data.fill(0);
  assert.equal([...f.itemizations()].length, 1387);
  // Every pass reads the one copy.
  assert.equal([...f.itemizationRecords()].length, 1387);
  f.close();
  assert.equal(f.id, "1721696");
});

test("many filings in a row reuse wasm memory", () => {
  const data = bytes(PAC);
  for (let i = 0; i < 50; i++) {
    const f = new wasm.Filing(data);
    for (const _ of f.itemizationRecords()) {
      // drain
    }
    f.close();
  }
});

test("close() mid-stream, then ERR_FILING_CLOSED", () => {
  const f = new wasm.Filing(bytes(PAC), { batchSize: 10 });
  const it = f.itemizations();
  it.next();
  f.close();
  assert.throws(() => it.next(), (e) => e.code === "ERR_FILING_CLOSED");
});

test("records and items survive structuredClone (postMessage from a worker)", () => {
  const f = new wasm.Filing(bytes(PAC));
  const [rec] = f.itemizationRecords();
  const [item] = f.itemizations();
  f.close();
  const clone = structuredClone(rec);
  assert.deepEqual(Object.keys(clone), ["rowType", "line", "values", "fields"]);
  assert.equal(clone.rowType, rec.rowType);
  assert.deepEqual(clone.values, rec.values);
  assert.deepEqual(clone.fields, rec.fields);
  assert.deepEqual(structuredClone(item), item);
  // The lazy itemization types it after close() too.
  assert.deepEqual(rec.itemization, item);
});

test("filings of 2 GiB or more: a RangeError with code ERR_FILING_TOO_LARGE, not a trap", () => {
  // Zero-filled and untouched, so this costs address space, not memory.
  const huge = new Uint8Array(2 ** 31);
  assert.throws(() => new wasm.Filing(huge), (e) => e instanceof RangeError && e.code === "ERR_FILING_TOO_LARGE" && /Node, Deno or Bun/.test(e.message));
  // readHeader copies only the start of the file into wasm memory, so it
  // reports the real problem (no header) instead.
  assert.throws(() => wasm.readHeader(huge), (e) => e.code === "FEC_PARSE");
  // And the instance is fine afterwards.
  const f = new wasm.Filing(bytes(PAC));
  assert.equal([...f.itemizations()].length, 1387);
  f.close();
});

test("readHeader on a big filing reads only its start", () => {
  const data = bytes(PAC);
  const big = new Uint8Array(2 ** 27);
  big.set(data);
  assert.deepEqual(wasm.readHeader(big), wasm.readHeader(data));
});
