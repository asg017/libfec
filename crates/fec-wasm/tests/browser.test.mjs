// What only the browser build does: bytes-only sources, and the wasm loaded
// before the module resolves (so the API is sync after import).
import { test } from "node:test";
import assert from "node:assert/strict";
import { pathToFileURL } from "node:url";
import { bytes, pyFixture, wasm } from "./helpers.mjs";

const PAC = pyFixture("1721696.fec");

test("paths and file: URLs are a TypeError with code ERR_UNSUPPORTED_SOURCE", () => {
  for (const source of [PAC, pathToFileURL(PAC)]) {
    for (const f of [() => wasm.open(source), () => wasm.read(source), () => wasm.readHeader(source)]) {
      assert.throws(f, (e) => e instanceof TypeError && e.code === "ERR_UNSUPPORTED_SOURCE" && /bytes, not paths/.test(e.message));
    }
  }
  // Still a TypeError for anything else.
  assert.throws(() => wasm.open(42), TypeError);
});

test("Uint8Array, Buffer and ArrayBuffer sources", () => {
  const data = bytes(PAC);
  const count = (src) => {
    const f = wasm.open(src);
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

test("the bytes are copied: mutating the source after open() changes nothing", () => {
  const data = bytes(PAC);
  const f = wasm.open(data, { id: "1721696" });
  data.fill(0);
  assert.equal([...f.itemizations()].length, 1387);
  f.close();
  assert.equal(f.id, "1721696");
});

test("many filings in a row reuse wasm memory", () => {
  const data = bytes(PAC);
  for (let i = 0; i < 50; i++) {
    const f = wasm.open(data);
    for (const _ of f.rows()) {
      // drain
    }
    f.close();
  }
});

test("close() mid-stream, then ERR_FILING_CLOSED", () => {
  const f = wasm.open(bytes(PAC), { batchSize: 10 });
  const it = f.itemizations()[Symbol.iterator]();
  it.next();
  f.close();
  assert.throws(() => it.next(), (e) => e.code === "ERR_FILING_CLOSED");
});

test("rows survive structuredClone (postMessage from a worker)", () => {
  const f = wasm.open(bytes(PAC));
  const [row] = f.itemizations();
  f.close();
  const clone = structuredClone(row);
  assert.equal(clone.rowType, row.rowType);
  assert.deepEqual(clone.values, row.values);
  assert.deepEqual(clone.itemization, row.itemization);
});
