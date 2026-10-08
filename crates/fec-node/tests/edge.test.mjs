// tests/fixtures/edge.fec, one edge case per row (see make-edge.mjs).
import { test } from "node:test";
import assert from "node:assert/strict";
import { MissingMappingError, open } from "../dist/index.js";
import { fixture } from "./helpers.mjs";

const EDGE = fixture("edge.fec", new URL("./fixtures/", import.meta.url));

function read(method, options) {
  const f = open(EDGE, options);
  try {
    return { rows: [...f[method]()], f };
  } finally {
    f.close();
  }
}

const byLine = (rows) => new Map(rows.map((r) => [r.line, r]));

test("records(): every field exactly, so no offset drifts", () => {
  const recs = byLine(read("records").rows);
  assert.equal(recs.get(3).fields[7], "Muñoz");
  assert.equal(recs.get(4).fields[7], "😀Smith");
  assert.equal(recs.get(4).fields[7].length, 7); // the emoji is 2 UTF-16 units
  assert.equal(recs.get(5).fields[7], "O�Neil"); // lossy cp1252 0x92
  // Fields after the non-ASCII ones are not shifted.
  for (const line of [3, 4, 5]) {
    assert.equal(recs.get(line).fields[8], "Ann");
    assert.equal(recs.get(line).fields[20], "50.00");
    assert.equal(recs.get(line).fields.length, 45);
  }
  assert.equal(recs.get(9).fields.length, 10);
  assert.deepEqual(recs.get(10).fields.slice(45), ["x", "y", "z"]);
  assert.equal(recs.get(12).rowType, "ZZ9");
  assert.equal(recs.get(20).fields[7], "Last");
  assert.equal(recs.get(20).fields[2], "E11");
});

test("quotes: a field wrapped in quotes loses one pair, but a quoted \\n or \\x1c still splits", () => {
  // fec-parser reads with CSV quoting off and then strips one pair of quotes
  // that wrap a whole field, reading "" inside as one " (lib.rs
  // unquote_record); a stray quote is kept and never spans separators.
  const recs = byLine(read("records").rows);
  assert.equal(recs.get(17).fields[7], 'O"Brien');
  assert.deepEqual(recs.get(18).fields.slice(7), ['"A', "B"]);
  assert.equal(recs.get(19).rowType, 'C"');
});

test("rows(): value rules on the edge rows", () => {
  const { rows } = read("rows", { unknownRows: "skip" });
  const r = byLine(rows);
  assert.equal(r.get(3).values.contributor_last_name, "Muñoz");
  assert.equal(r.get(6).values.contribution_date, null);
  assert.equal(r.get(7).values.contribution_amount, "1,000.00");
  assert.equal(r.get(8).values.contribution_amount, null);
  assert.equal(r.get(9).values.contributor_first_name, "Ann");
  assert.equal(r.get(9).values.contribution_amount, null);
  assert.deepEqual(r.get(10).extraFields, ["x", "y", "z"]);
  assert.equal(Object.keys(r.get(10).values).length, 45);
  // TEXT rows: the first column is rec_type (Q16).
  assert.equal(r.get(11).rowType, "TEXT");
  assert.equal(r.get(11).values.rec_type, "TEXT");
  assert.ok(!("form_type" in r.get(11).values));
  assert.equal(r.get(16).rowType, "F3S");
  assert.equal(r.get(20).values.contributor_last_name, "Last");
});

test("rows(): invalidValues lists exactly the garbage date and amount", () => {
  const { f } = read("rows", { unknownRows: "skip" });
  assert.deepEqual(f.invalidValues, [
    { line: 6, column: "contribution_date", raw: "20230230" },
    { line: 7, column: "contribution_amount", raw: "1,000.00" },
  ]);
  assert.equal(f.invalidValueCount, 2);
  assert.deepEqual(f.skippedRows, [
    { rowType: "ZZ9", line: 12 },
    { rowType: 'C"', line: 19 },
  ]);
});

test("rows(): an unmapped row throws by default", () => {
  assert.throws(() => read("rows"), (e) => e instanceof MissingMappingError && e.line === 12);
});

test("itemizations(): types, skips, garbage as the parser gives it", () => {
  const { rows, f } = read("itemizations");
  const r = byLine(rows);
  assert.deepEqual(
    rows.map((x) => [x.line, x.itemization.type]),
    [
      [3, "ScheduleA"], [4, "ScheduleA"], [5, "ScheduleA"], [6, "ScheduleA"], [7, "ScheduleA"],
      [8, "ScheduleA"], [9, "ScheduleA"], [10, "ScheduleA"], [11, "TextRecord"],
      [13, "ScheduleA3L"], [14, "ScheduleC"], [15, "ScheduleC1"],
      [17, "ScheduleA"], [18, "ScheduleA"], [20, "ScheduleA"],
    ],
  );
  // ZZ9 (unmapped), F3S (no struct) and the split-off `C"` line are skipped, no error.
  assert.deepEqual(f.skippedRows, []);
  assert.equal(r.get(3).itemization.contributor.name.last_name, "Muñoz");
  assert.equal(r.get(4).itemization.contributor.name.last_name, "😀Smith");
  assert.equal(r.get(5).itemization.contributor.name.last_name, "O�Neil");
  // T3: garbage in the typed record is the parser's value; `values` keeps the raw.
  assert.equal(r.get(7).itemization.contribution_amount, 0);
  assert.equal(r.get(7).values.contribution_amount, "1,000.00");
  assert.equal(r.get(6).itemization.contribution_date, null);
  assert.equal(r.get(20).itemization.contributor.name.last_name, "Last");
});
