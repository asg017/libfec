// The Node and Python bindings agree, value for value, on every fixture.
// The Python side is committed JSON (tests/fixtures/*.typed.json.gz, from
// make-typed-json.sh): each mapped row's values and typed record, the typed
// cover and the cover row.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { gunzipSync } from "node:zlib";
import { columns, open } from "../dist/index.js";
import { fixture } from "./helpers.mjs";

const NODE_FIXTURES = new URL("./fixtures/", import.meta.url);
const FILES = [
  ["1921705", fixture("1921705.fec")],
  ["1721696", fixture("1721696.fec")],
  ["1913493", fixture("1913493.fec")],
  ["1913562", fixture("1913562.fec")],
  ["1923816", fixture("1923816.fec")],
  ["edge", fixture("edge.fec", NODE_FIXTURES)],
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

for (const [name, path] of FILES) {
  test(`parity ${name}: rows() values`, () => {
    const py = python(name);
    const f = open(path, { unknownRows: "skip" });
    try {
      sameValues(f.coverRow.values, py.cover_row, f.coverRow.rowType, f.fecVersion, "cover row");
      const rows = [...f.rows()];
      assert.deepEqual(rows.map((r) => [r.line, r.rowType]), py.rows.map((r) => [r.line, r.row_type]));
      let garbage = 0;
      rows.forEach((r, i) => {
        garbage += sameValues(r.values, py.rows[i].values, r.rowType, f.fecVersion, `line ${r.line}`);
      });
      assert.equal(garbage, name === "edge" ? 1 : 0);
    } finally {
      f.close();
    }
  });

  test(`parity ${name}: itemizations() and the typed cover`, () => {
    const py = python(name);
    const f = open(path);
    try {
      // Python's dump already tags each record with its class name as `type`,
      // and its cover is the flat struct: the JS shape (T5, T12, T13).
      assert.deepEqual(f.cover, py.cover);
      const typed = py.rows.filter((r) => r.itemization !== null);
      const rows = [...f.itemizations()];
      assert.deepEqual(rows.map((r) => r.line), typed.map((r) => r.line));
      rows.forEach((r, i) => {
        assert.deepEqual(r.itemization, typed[i].itemization, `line ${r.line}`);
        // Key order too (deepEqual ignores it): type first, then serde order.
        assert.deepEqual(Object.keys(r.itemization), Object.keys(typed[i].itemization), `line ${r.line}`);
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

const rowsOf = (path) => {
  const f = open(path);
  try {
    return [...f.rows()];
  } finally {
    f.close();
  }
};

test("test_row_types_in_file_order", () => {
  assert.deepEqual(rowsOf(SAMPLE).map((r) => r.rowType), ROW_TYPES);
});

test("test_lines", () => {
  assert.deepEqual(rowsOf(SAMPLE).map((r) => r.line), Array.from({ length: 20 }, (_, i) => i + 3));
});

test("test_typed_values_through_reader", () => {
  const [row] = rowsOf(SAMPLE);
  assert.equal(row.values.contribution_amount, 500);
  assert.equal(row.values.contribution_date, "2025-07-07");
});

test("test_pac_row_count", () => {
  assert.equal(rowsOf(PAC).length, 1387);
});

test("test_f99_has_no_rows", () => {
  const f = open(fixture("1913493.fec"));
  assert.equal(f.coverSummary.formType, "F99");
  assert.deepEqual([...f.rows()], []);
});

test("test_cover_dates / test_cover_fields", () => {
  const f = open(SAMPLE);
  assert.equal(f.coverSummary.coverageFromDate, "2025-07-01");
  assert.equal(f.coverSummary.coverageThroughDate, "2025-09-30");
  assert.deepEqual(Object.keys(f.coverSummary).sort(), [
    "coverageFromDate", "coverageThroughDate", "filerId", "filerName", "formType", "reportCode",
  ]);
  f.close();
});

test("test_cover_row", () => {
  const s = open(SAMPLE);
  assert.equal(s.coverRow.rowType, "F3N");
  assert.equal(s.coverRow.line, 2);
  assert.equal(s.coverRow.values.filer_committee_id_number, "C00900860");
  const p = open(PAC);
  assert.equal(p.coverRow.values.col_a_total_receipts, 83741.93);
  assert.equal(p.coverRow.values.coverage_from_date, "2023-07-01");
  assert.equal(p.coverRow.fields[0], "F3XN");
  assert.equal(p.coverRow.line, 2);
});

test("test_cover_row_keys_count", () => {
  assert.equal(Object.keys(open(PAC).coverRow.values).length, 123);
});

test("test_id", () => {
  assert.equal(open(SAMPLE).id, "1921705");
  assert.equal(open(new Uint8Array(readFileSync(SAMPLE))).id, null);
});
