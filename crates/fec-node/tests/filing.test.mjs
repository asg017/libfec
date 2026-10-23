// Smoke test of the Filing API (ticket 02); tickets 04/05 widen it.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import { Filing, Record, Unknown, FecError, FecParseError, MissingMappingError } from "../dist/index.js";
import { fixture, LEGACY_FIXTURES } from "./helpers.mjs";

const PAC = fixture("1721696.fec");
/** 31 typed rows and 2 `SI` rows with no typed class. */
const MIXED = fixture("3.00_13801.fec", LEGACY_FIXTURES);
const EDGE = fixture("edge.fec", new URL("./fixtures/", import.meta.url));

test("every source kind opens; header, cover, filer attributes up front", () => {
  const bytes = readFileSync(PAC);
  const sources = [
    PAC,
    pathToFileURL(PAC),
    new Uint8Array(bytes),
    bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength),
  ];
  for (const src of sources) {
    const f = new Filing(src);
    assert.equal(f.fecVersion, "8.4");
    assert.equal(f.header.fecVersion, "8.4");
    assert.equal(f.sourceLength, bytes.length);
    assert.equal(f.formType, "F3XN");
    assert.equal(f.filerId, "C00016683");
    assert.equal(f.filerName, "PFIZER INC. PAC");
    assert.equal(f.reportCode, "M8");
    assert.equal(f.coverageFromDate, "2023-07-01");
    assert.equal(f.coverageThroughDate, "2023-07-31");
    assert.equal(f.cover.type, "Form3X");
    assert.equal(f.cover, f.cover);
    assert.ok(f.coverRecord instanceof Record);
    assert.equal(f.coverRecord, f.coverRecord);
    assert.equal(f.coverRecord.rowType, "F3XN");
    assert.equal(f.coverRecord.line, 2);
    assert.equal(f.closed, false);
    f.close();
  }
  assert.equal(new Filing(PAC).id, "1721696");
  assert.equal(new Filing(new Uint8Array(bytes)).id, null);
  assert.equal(new Filing(new Uint8Array(bytes), { id: "x" }).id, "x");
  const d = new Filing(PAC, { dates: "date" });
  assert.deepEqual(d.coverageFromDate, new Date("2023-07-01T00:00:00Z"));
});

test("errors at construction", () => {
  assert.throws(() => new Filing("nope.fec"), { code: "ENOENT", syscall: "open" });
  assert.throws(() => new Filing(new TextEncoder().encode("garbage")), FecParseError);
  assert.throws(() => new Filing(PAC, { unknownRows: "skip" }), TypeError);
  assert.throws(() => new Filing(PAC, { dates: "nope" }), TypeError);
  assert.throws(() => new Filing(42), TypeError);
});

test("each call is a new pass; passes interleave", () => {
  const f = new Filing(PAC, { batchSize: 100 });
  const a = [...f.itemizationRecords()];
  const b = [...f.itemizationRecords()];
  assert.equal(a.length, 1387);
  assert.deepEqual(a.map((r) => r.fields), b.map((r) => r.fields));
  assert.equal([...f.itemizations()].length, 1387);
  // Interleaved: two live passes over one source.
  const items = f.itemizations();
  const records = f.itemizationRecords();
  let n = 0;
  for (;;) {
    const x = items.next();
    const y = records.next();
    assert.equal(x.done, y.done);
    if (x.done) break;
    assert.equal(x.value.form_type, y.value.rowType);
    n++;
  }
  assert.equal(n, 1387);
  assert.throws(() => f.itemizations("ScheduleA"), TypeError);
  assert.throws(() => f.itemizationRecords({}), TypeError);
});

test("Unknown for untyped rows, one-to-one with itemizationRecords()", () => {
  const f = new Filing(MIXED);
  const items = [...f.itemizations()];
  const records = [...f.itemizationRecords()];
  assert.equal(items.length, 33);
  assert.equal(records.length, 33);
  const unknown = items.filter((i) => i instanceof Unknown);
  assert.equal(unknown.length, 2);
  for (let i = 0; i < items.length; i++) {
    const item = items[i];
    const rec = records[i];
    if (item.type === "Unknown") {
      assert.equal(item.formType, "SI");
      assert.equal(item.formType, rec.rowType);
      assert.deepEqual(item.record.fields, rec.fields);
      assert.equal(item.record.line, rec.line);
      assert.deepEqual(item.toJSON(), { type: "Unknown", formType: "SI", values: rec.values });
      assert.equal(rec.itemization, null);
    } else {
      assert.equal(item.form_type, rec.rowType);
      // The lazy itemization types the same record the same way.
      assert.deepEqual(rec.itemization, item);
    }
  }
});

test("Record: lazy itemization, plain data, toJSON", () => {
  const f = new Filing(PAC);
  const [rec] = f.itemizationRecords();
  const it = rec.itemization;
  assert.equal(it.type, "ScheduleA");
  assert.equal(it.contributor.name.last_name, "Aaronson");
  // Typed anew on every access.
  assert.notEqual(rec.itemization, it);
  assert.deepEqual(rec.itemization, it);
  // Private state stays out of spreads and clones.
  assert.deepEqual(Object.keys({ ...rec }), ["rowType", "line", "values", "fields"]);
  assert.deepEqual(Object.keys(structuredClone(rec)), ["rowType", "line", "values", "fields"]);
  assert.deepEqual(Object.keys(rec.toJSON()), ["rowType", "line", "values"]);
  assert.equal(rec.get("contribution_amount"), 104.17);
});

test("close() stops a running pass; later calls throw at once", () => {
  for (const make of [(f) => f.itemizations(), (f) => f.itemizationRecords()]) {
    const f = new Filing(PAC, { batchSize: 10 });
    const g = make(f);
    assert.equal(g.next().done, false);
    f.close();
    f.close();
    assert.equal(f.closed, true);
    assert.throws(() => g.next(), (e) => e instanceof FecError && e.code === "ERR_FILING_CLOSED");
    assert.throws(() => make(f), { code: "ERR_FILING_CLOSED" });
    // The up-front data stays readable.
    assert.equal(f.cover.type, "Form3X");
  }
  const f = new Filing(PAC);
  f[Symbol.dispose]();
  assert.equal(f.closed, true);
});

test("a row with no column mapping throws MissingMappingError in both views", () => {
  const f = new Filing(EDGE);
  for (const pass of [() => [...f.itemizations()], () => [...f.itemizationRecords()]]) {
    assert.throws(pass, (e) => e instanceof MissingMappingError && e.rowType === "ZZ9");
  }
  // The rows before it are still yielded.
  const seen = [];
  assert.throws(() => {
    for (const r of f.itemizationRecords()) seen.push(r.rowType);
  }, MissingMappingError);
  assert.ok(seen.length > 0);
});
