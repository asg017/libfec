// Records (js/record.ts): building them from native batches and raw fields,
// the value rules, the Record class and its lazy itemization.
import { test } from "node:test";
import assert from "node:assert/strict";
import * as native from "../native/native.js";
import { Filing, MissingMappingError, Record } from "../dist/index.js";
import { RecordBuilder, parseAmount, parseDate } from "../dist/record.js";
import { fixture, LEGACY_FIXTURES } from "./helpers.mjs";

const SAMPLE = fixture("1921705.fec");
const PAC = fixture("1721696.fec");

/** Every record of a file through a RecordBuilder, straight off native batches. */
function build(path, { dates = "iso", codegen } = {}) {
  const src = native.NativeSource.openPath(path);
  const r = src.reader(false);
  const b = new RecordBuilder(r.fecVersion, r.header.nameDelimiter, dates, { codegen });
  const out = [];
  for (let batch; (batch = r.nextBatch(1024)); ) out.push(...b.build(batch));
  src.close();
  return out;
}

function records(path, options) {
  const f = new Filing(path, options);
  try {
    return [...f.itemizationRecords()];
  } finally {
    f.close();
  }
}

test("values: typed by column kind, in column order", () => {
  const [first] = records(SAMPLE);
  assert.equal(first.values.contribution_amount, 500);
  assert.equal(first.values.contribution_date, "2025-07-07");
  const sa = records(PAC).find((r) => r.rowType === "SA11AI");
  assert.equal(sa.values.contributor_last_name, "Aaronson");
  assert.equal(sa.values.contribution_amount, 104.17);
  assert.equal(sa.values.contribution_aggregate, 1458.38);
  assert.equal(sa.values.contribution_date, "2023-07-14");
  assert.equal(sa.values.transaction_id, "2023071716378-1066");
  assert.deepEqual(Object.keys(sa.values), native.schema("SA11AI", "8.4").map((c) => c.name));
});

test("Record: own data properties, methods on the prototype, get(), toJSON()", () => {
  const [rec] = records(SAMPLE);
  assert.ok(rec instanceof Record);
  assert.deepEqual(Object.keys(rec), ["rowType", "line", "values", "fields"]);
  assert.equal(rec.rowType, "SA11AI");
  assert.equal(rec.line, 3);
  assert.equal(rec.fields[0], rec.rowType);
  assert.equal(rec.get("contribution_amount"), 500);
  assert.equal(rec.get("nope"), undefined);
  assert.equal(rec.get("toString"), undefined);
  assert.deepEqual(rec.extraFields, []);
  // toJSON: no fields, and no itemization (it's a lazy getter, not data).
  assert.deepEqual(Object.keys(rec.toJSON()), ["rowType", "line", "values"]);
  assert.deepEqual(JSON.parse(JSON.stringify(rec)), { rowType: "SA11AI", line: 3, values: JSON.parse(JSON.stringify(rec.values)) });
  for (const k of ["itemization", "extraFields", "get", "toJSON"]) {
    assert.ok(!Object.hasOwn(rec, k), k);
  }
});

test("Record: spreads and structured clones are plain data", () => {
  const [rec] = records(SAMPLE);
  const spread = { ...rec };
  assert.deepEqual(Object.keys(spread), ["rowType", "line", "values", "fields"]);
  assert.equal(spread.itemization, undefined);
  // postMessage to/from a worker: every data field is an own enumerable
  // property; the methods and the private fecVersion are left behind.
  const clone = structuredClone(rec);
  assert.equal(Object.getPrototypeOf(clone), Object.prototype);
  assert.deepEqual(Object.keys(clone), ["rowType", "line", "values", "fields"]);
  for (const k of ["rowType", "line", "values", "fields"]) assert.deepEqual(clone[k], rec[k], k);
});

test("Record.itemization: lazy, new each time, equal to itemizations() at the same index", () => {
  for (const path of [SAMPLE, PAC]) {
    const f = new Filing(path);
    const items = [...f.itemizations()];
    const recs = [...f.itemizationRecords()];
    assert.equal(items.length, recs.length);
    recs.forEach((rec, i) => {
      const it = rec.itemization;
      assert.deepEqual(it, items[i], `line ${rec.line}`);
      assert.deepEqual(Object.keys(it), Object.keys(items[i]), `line ${rec.line}: key order`);
    });
    const [rec] = recs;
    assert.notEqual(rec.itemization, rec.itemization);
    f.close();
  }
  const sa = records(PAC).find((r) => r.rowType === "SA11AI").itemization;
  assert.equal(sa.type, "ScheduleA");
  assert.ok(!("family" in sa));
  assert.equal(sa.contributor.name.last_name, "Aaronson");
  assert.equal(sa.form_type, "SA11AI");
});

test("Record.itemization: null for a summary record and a schedule with no type", () => {
  // 1913562 (F1A) has F1S rows; 3.00_13801 has SI rows.
  const f1s = records(fixture("1913562.fec")).filter((r) => r.rowType === "F1S");
  assert.ok(f1s.length > 0);
  for (const r of f1s) assert.equal(r.itemization, null);
  const si = records(fixture("3.00_13801.fec", LEGACY_FIXTURES)).filter((r) => r.rowType === "SI");
  assert.equal(si.length, 2);
  for (const r of si) assert.equal(r.itemization, null);
});

test("Record.itemization: a filing closed since still types its records", () => {
  const f = new Filing(PAC);
  const [rec] = f.itemizationRecords();
  f.close();
  assert.equal(rec.itemization.type, "ScheduleA");
});

test("Record.itemization: name-delimited legacy names split the way the typed pass does", () => {
  // 3.00 uses a name sub-delimiter (`^`) the lazy getter needs too.
  const path = fixture("3.00_13801.fec", LEGACY_FIXTURES);
  const f = new Filing(path);
  assert.notEqual(f.header.nameDelimiter, null);
  const items = [...f.itemizations()];
  const recs = [...f.itemizationRecords()];
  recs.forEach((rec, i) => {
    if (items[i].type !== "Unknown") assert.deepEqual(rec.itemization, items[i], `line ${rec.line}`);
  });
  f.close();
});

test("codegen fallback gives identical records", () => {
  for (const path of [SAMPLE, PAC]) {
    const a = build(path, { codegen: true });
    const b = build(path, { codegen: false });
    assert.equal(JSON.stringify(a), JSON.stringify(b));
    assert.deepEqual(a.map((r) => r.fields), b.map((r) => r.fields));
  }
});

test("dates: 'date' gives Date at UTC midnight, in values only", () => {
  const [rec] = records(SAMPLE, { dates: "date" });
  assert.ok(rec.values.contribution_date instanceof Date);
  assert.equal(rec.values.contribution_date.toISOString(), "2025-07-07T00:00:00.000Z");
  assert.equal(rec.itemization.contribution_date, "2025-07-07");
  const f = new Filing(SAMPLE, { dates: "date" });
  assert.equal([...f.itemizations()][0].contribution_date, "2025-07-07");
  assert.ok(f.coverRecord.values.coverage_from_date instanceof Date);
  f.close();
});

test("amount grammar matches Rust's f64 parse", () => {
  const ok = {
    "104.17": 104.17, "-5": -5, "+5": 5, ".5": 0.5, "5.": 5, "1e3": 1000, "1E-3": 0.001,
    "00012": 12, inf: Infinity, "-Infinity": -Infinity, "+INF": Infinity,
  };
  for (const [s, n] of Object.entries(ok)) assert.equal(parseAmount(s), n, s);
  assert.ok(Number.isNaN(parseAmount("NaN")));
  for (const s of ["0x10", "0b1", "1,000.00", "$5", ".", "e5", "1e", "1_000", "Infinityx", "٣"]) {
    assert.equal(parseAmount(s), undefined, s);
  }
});

test("date validity: 8 digits forming a real calendar date, years 1..=9999", () => {
  assert.deepEqual(parseDate("20240229"), [2024, 2, 29]);
  assert.deepEqual(parseDate("00010101"), [1, 1, 1]);
  assert.deepEqual(parseDate("99991231"), [9999, 12, 31]);
  for (const s of ["00000101", "00001231", "-0010101", "20230229", "20230230", "20231301", "20230100", "2023011", "+20230101", "2023-01-01", "202301011", "19000229"]) {
    assert.equal(parseDate(s), undefined, s);
  }
  assert.deepEqual(parseDate("20000229"), [2000, 2, 29]);
});

test("value rules: blanks, garbage, short rows, extra fields", () => {
  const b = new RecordBuilder("8.4", null, "iso");
  const fields = native.schema("SA11AI", "8.4").map(() => "");
  fields[0] = "SA11AI";
  fields[7] = "  O'BRIEN ";
  fields[19] = "20230230";
  fields[20] = " 1,000.00 ";
  fields[21] = "   ";
  const rec = b.record(fields, 9);
  assert.equal(rec.line, 9);
  assert.equal(rec.values.contributor_last_name, "  O'BRIEN "); // text: untrimmed
  assert.equal(rec.values.contributor_first_name, "");
  assert.equal(rec.values.contribution_date, null); // invalid date
  assert.equal(rec.values.contribution_amount, " 1,000.00 "); // unparseable amount: the raw string
  assert.equal(rec.values.contribution_aggregate, null); // blank amount
  const short = b.record(["SA11AI", "C001"], 10);
  assert.equal(short.values.contribution_amount, null);
  assert.equal(short.values.filer_committee_id_number, "C001");
  assert.equal(short.values.transaction_id, null);
  assert.deepEqual(b.record([...fields.slice(0, 45), "", "x"], 11).extraFields, ["", "x"]);
  assert.deepEqual(b.record([...fields.slice(0, 45), "", ""], 12).extraFields, []);
  // Paper row types come padded; the record's rowType is trimmed.
  assert.equal(b.record(["SA11AI  ", "C001"], 13).rowType, "SA11AI");
});

test("an unmapped row type comes out as its MissingMappingError", () => {
  const b = new RecordBuilder("8.4", null, "iso");
  const e = b.record(["ZZ9", "x"], 4);
  assert.ok(e instanceof MissingMappingError);
  assert.equal(e.rowType, "ZZ9");
  assert.equal(e.fecVersion, "8.4");
  assert.equal(e.line, 4);
});
