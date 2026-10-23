// The Node and Python bindings agree, value for value, on every fixture.
// The Python side is committed JSON (tests/fixtures/*.typed.json.gz, from
// make-typed-json.sh running dump_typed.py on `libfec.Filing`): the filer
// attributes, the typed cover and the cover record, and both views of the
// records after the cover, `itemizations` and `itemization_records`.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { gunzipSync } from "node:zlib";
import { columns, Filing, MissingMappingError, Unknown } from "../dist/index.js";
import { fixture } from "./helpers.mjs";

const NODE_FIXTURES = new URL("./fixtures/", import.meta.url);
const FILES = [
  ["1921705", fixture("1921705.fec")],
  ["1721696", fixture("1721696.fec")],
  ["1913493", fixture("1913493.fec")],
  ["1913562", fixture("1913562.fec")],
  ["1923816", fixture("1923816.fec")],
  ["edge", fixture("edge.fec", NODE_FIXTURES)],
  ["unknown-cover", fixture("unknown-cover.fec", NODE_FIXTURES)],
];

function python(name) {
  const gz = readFileSync(new URL(`${name}.typed.json.gz`, NODE_FIXTURES));
  return JSON.parse(gunzipSync(gz).toString("utf8"));
}

/**
 * Python's values next to ours. The one expected difference (plans/nodejs
 * 00-decisions.md Q4): an invalid date is `null` here and the raw string in
 * Python. Returns how many such dates it saw.
 */
function sameValues(js, py, rowType, fecVersion, where) {
  const kinds = new Map(columns(rowType, fecVersion).map((c) => [c.name, c.kind]));
  let garbageDates = 0;
  assert.deepEqual(Object.keys(js), Object.keys(py), `${where}: keys`);
  for (const [k, v] of Object.entries(py)) {
    if (kinds.get(k) === "date" && typeof v === "string" && !/^\d{4}-\d{2}-\d{2}$/.test(v)) {
      assert.equal(js[k], null, `${where}: ${k} (garbage date)`);
      garbageDates++;
      continue;
    }
    assert.deepEqual(js[k], v, `${where}: ${k}`);
  }
  return garbageDates;
}

/** A Python typed dict (or Unknown) against ours; returns the garbage dates seen. */
function sameTyped(js, py, fecVersion, where) {
  if (py.type === "Unknown") {
    assert.ok(js instanceof Unknown, `${where}: expected an Unknown, got ${js.type}`);
    const json = js.toJSON();
    assert.deepEqual(Object.keys(json), ["type", "formType", "values"], where);
    assert.equal(json.type, "Unknown", where);
    assert.equal(json.formType, py.form_type, where);
    assert.equal(js.record.rowType, py.form_type, where);
    return sameValues(json.values, py.values, py.form_type, fecVersion, where);
  }
  assert.ok(!(js instanceof Unknown), `${where}: expected ${py.type}, got an Unknown (${js.formType})`);
  assert.deepEqual(js, py, where);
  // Key order too (deepEqual ignores it): type first, then serde order.
  assert.deepEqual(Object.keys(js), Object.keys(py), `${where}: key order`);
  return 0;
}

/**
 * Drain a JS pass: everything it yields, and the error it ends with (if any).
 * Python's dump records a MissingMappingError in place and carries on (its
 * iterators do); a JS generator is done once it throws. So the JS side must
 * match Python's entries up to the first error, then throw that same error,
 * then be done.
 */
function drain(iterator) {
  const items = [];
  for (;;) {
    let step;
    try {
      step = iterator.next();
    } catch (error) {
      return { items, error, after: iterator.next() };
    }
    if (step.done) return { items, error: null };
    items.push(step.value);
  }
}

/** Python's entries before its first error, and that error (or null). */
function untilError(entries) {
  const i = entries.findIndex((e) => "error" in e);
  return i < 0 ? { entries, error: null } : { entries: entries.slice(0, i), error: entries[i] };
}

function sameEnd(js, py, where) {
  if (py.error === null) {
    assert.equal(js.error, null, `${where}: unexpected ${js.error}`);
    return;
  }
  assert.equal(py.error.error, "MissingMappingError");
  assert.ok(js.error instanceof MissingMappingError, `${where}: expected MissingMappingError, got ${js.error}`);
  assert.equal(js.error.rowType, py.error.row_type, `${where}: error rowType`);
  assert.equal(js.error.line, py.error.line, `${where}: error line`);
  assert.deepEqual(js.after, { value: undefined, done: true }, `${where}: done after the throw`);
}

/** How many garbage dates each fixture's records hold (the edge row at line 6). */
const GARBAGE_DATES = { edge: 1 };

for (const [name, path] of FILES) {
  test(`parity ${name}: filer attributes, cover, coverRecord`, () => {
    const py = python(name);
    const f = new Filing(path);
    try {
      assert.deepEqual(
        {
          form_type: f.formType,
          filer_id: f.filerId,
          filer_name: f.filerName,
          report_code: f.reportCode,
          coverage_from_date: f.coverageFromDate,
          coverage_through_date: f.coverageThroughDate,
        },
        py.filer,
      );
      sameTyped(f.cover, py.cover, f.fecVersion, "cover");
      assert.equal(f.coverRecord.line, py.cover_record.line);
      assert.equal(f.coverRecord.rowType, py.cover_record.row_type);
      sameValues(f.coverRecord.values, py.cover_record.values, f.coverRecord.rowType, f.fecVersion, "cover record");
    } finally {
      f.close();
    }
  });

  test(`parity ${name}: itemizationRecords()`, () => {
    const py = untilError(python(name).itemization_records);
    const f = new Filing(path);
    try {
      const js = drain(f.itemizationRecords());
      assert.deepEqual(
        js.items.map((r) => [r.line, r.rowType]),
        py.entries.map((r) => [r.line, r.row_type]),
      );
      let garbage = 0;
      js.items.forEach((r, i) => {
        garbage += sameValues(r.values, py.entries[i].values, r.rowType, f.fecVersion, `line ${r.line}`);
      });
      assert.equal(garbage, GARBAGE_DATES[name] ?? 0);
      sameEnd(js, py, name);
    } finally {
      f.close();
    }
  });

  test(`parity ${name}: itemizations()`, () => {
    const py = untilError(python(name).itemizations);
    const f = new Filing(path);
    try {
      const js = drain(f.itemizations());
      assert.equal(js.items.length, py.entries.length);
      js.items.forEach((item, i) => sameTyped(item, py.entries[i], f.fecVersion, `item ${i}`));
      sameEnd(js, py, name);
    } finally {
      f.close();
    }
  });

  test(`parity ${name}: the two views are one-to-one; lazy Record.itemization`, () => {
    const py = python(name);
    // Python's two views line up, errors included...
    assert.equal(py.itemizations.length, py.itemization_records.length);
    py.itemizations.forEach((item, i) => {
      const rec = py.itemization_records[i];
      assert.equal("error" in item, "error" in rec, `entry ${i}`);
      if ("error" in item) assert.deepEqual(item, rec);
      else if (item.type === "Unknown") assert.equal(item.form_type, rec.row_type);
    });
    // ...and so do ours.
    const f = new Filing(path);
    try {
      const items = drain(f.itemizations());
      const records = drain(f.itemizationRecords());
      assert.equal(items.items.length, records.items.length);
      assert.equal(items.error?.line, records.error?.line);
      const n = records.items.length;
      records.items.forEach((rec, i) => {
        const item = items.items[i];
        if (item instanceof Unknown) {
          assert.equal(item.record.line, rec.line);
          assert.deepEqual(item.record.fields, rec.fields);
          assert.equal(rec.itemization, null, `line ${rec.line}: no typed class`);
        } else {
          assert.equal(item.form_type, rec.rowType, `line ${rec.line}`);
          // The lazy getter types one record the way the typed pass does:
          // every one in a small file, every 50th in a big one.
          if (n < 200 || i % 50 === 0) assert.deepEqual(rec.itemization, item, `line ${rec.line}`);
        }
      });
    } finally {
      f.close();
    }
  });
}

// Ported from crates/fec-py/tests/test_reader.py (python-phase2), by name.

const SAMPLE = fixture("1921705.fec");
const PAC = fixture("1721696.fec");
const ROW_TYPES = ["SA11AI", ...Array(13).fill("SA11C"), "SA11D", ...Array(5).fill("SB17")];

const recordsOf = (path) => {
  const f = new Filing(path);
  try {
    return [...f.itemizationRecords()];
  } finally {
    f.close();
  }
};

test("test_row_types_in_file_order", () => {
  assert.deepEqual(recordsOf(SAMPLE).map((r) => r.rowType), ROW_TYPES);
});

test("test_lines", () => {
  assert.deepEqual(recordsOf(SAMPLE).map((r) => r.line), Array.from({ length: 20 }, (_, i) => i + 3));
});

test("test_typed_values_through_reader", () => {
  const [row] = recordsOf(SAMPLE);
  assert.equal(row.values.contribution_amount, 500);
  assert.equal(row.values.contribution_date, "2025-07-07");
});

test("test_pac_row_count", () => {
  assert.equal(recordsOf(PAC).length, 1387);
});

test("test_f99_has_no_rows", () => {
  const f = new Filing(fixture("1913493.fec"));
  assert.equal(f.formType, "F99");
  assert.deepEqual([...f.itemizationRecords()], []);
  assert.deepEqual([...f.itemizations()], []);
});

test("test_cover_dates / test_cover_fields", () => {
  const f = new Filing(SAMPLE);
  assert.equal(f.coverageFromDate, "2025-07-01");
  assert.equal(f.coverageThroughDate, "2025-09-30");
  assert.equal(f.formType, "F3N");
  assert.equal(f.filerId, "C00900860");
  assert.equal(f.reportCode, "Q3");
  f.close();
});

test("test_cover_record", () => {
  const s = new Filing(SAMPLE);
  assert.equal(s.coverRecord.rowType, "F3N");
  assert.equal(s.coverRecord.line, 2);
  assert.equal(s.coverRecord.values.filer_committee_id_number, "C00900860");
  const p = new Filing(PAC);
  assert.equal(p.coverRecord.values.col_a_total_receipts, 83741.93);
  assert.equal(p.coverRecord.values.coverage_from_date, "2023-07-01");
  assert.equal(p.coverRecord.fields[0], "F3XN");
  assert.equal(p.coverRecord.line, 2);
});

test("test_cover_record_keys_count", () => {
  assert.equal(Object.keys(new Filing(PAC).coverRecord.values).length, 123);
});

test("test_id", () => {
  assert.equal(new Filing(SAMPLE).id, "1921705");
  assert.equal(new Filing(new Uint8Array(readFileSync(SAMPLE))).id, null);
});
