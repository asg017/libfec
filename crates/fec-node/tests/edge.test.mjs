// tests/fixtures/edge.fec, one edge case per row (see make-edge.mjs).
import { test } from "node:test";
import assert from "node:assert/strict";
import { Filing, MissingMappingError, Unknown } from "../dist/index.js";
import { fixture } from "./helpers.mjs";

const EDGE = fixture("edge.fec", new URL("./fixtures/", import.meta.url));

/**
 * Drain a pass of edge.fec. It ends at line 20, the `C"…` line split off
 * the quoted \n at line 19, which has no column mapping: everything before it
 * is yielded, then that MissingMappingError, then the pass is done.
 */
function drain(view, options) {
  const f = new Filing(EDGE, options);
  try {
    const it = f[view]();
    const out = [];
    assert.throws(
      () => {
        for (const x of it) out.push(x);
      },
      (e) => e instanceof MissingMappingError && e.rowType === 'C"' && e.line === 20 && e.fecVersion === "8.5",
    );
    assert.equal(it.next().done, true);
    return out;
  } finally {
    f.close();
  }
}

const byLine = (records) => new Map(records.map((r) => [r.line, r]));

test("itemizationRecords(): every field exactly, so no offset drifts", () => {
  const recs = drain("itemizationRecords");
  assert.deepEqual(recs.map((r) => r.line), Array.from({ length: 17 }, (_, i) => i + 3));
  const r = byLine(recs);
  assert.equal(r.get(3).fields[7], "Muñoz");
  assert.equal(r.get(4).fields[7], "😀Smith");
  assert.equal(r.get(4).fields[7].length, 7); // the emoji is 2 UTF-16 units
  assert.equal(r.get(5).fields[7], "O�Neil"); // lossy cp1252 0x92
  // Fields after the non-ASCII ones are not shifted.
  for (const line of [3, 4, 5]) {
    assert.equal(r.get(line).fields[8], "Ann");
    assert.equal(r.get(line).fields[20], "50.00");
    assert.equal(r.get(line).fields.length, 45);
  }
  assert.equal(r.get(9).fields.length, 10);
  assert.deepEqual(r.get(10).fields.slice(45), ["x", "y", "z"]);
  assert.equal(r.get(18).fields[7], "Last");
  assert.equal(r.get(18).fields[2], "E11");
});

test("quotes: a field wrapped in quotes loses one pair, but a quoted \\n or \\x1c still splits", () => {
  // fec-parser reads with CSV quoting off and then strips one pair of quotes
  // that wrap a whole field, reading "" inside as one " (lib.rs
  // unquote_record); a stray quote is kept and never spans separators.
  const r = byLine(drain("itemizationRecords"));
  assert.equal(r.get(17).fields[7], 'O"Brien');
  assert.deepEqual(r.get(19).fields.slice(7), ['"A', "B"]);
  // The rest of the quoted field starts line 20 (`C"`), the MissingMappingError
  // drain() checks.
});

test("itemizationRecords(): value rules on the edge rows", () => {
  const r = byLine(drain("itemizationRecords"));
  assert.equal(r.get(3).values.contributor_last_name, "Muñoz");
  assert.equal(r.get(6).values.contribution_date, null); // garbage date
  assert.equal(r.get(7).values.contribution_amount, "1,000.00"); // garbage amount keeps its raw text
  assert.equal(r.get(8).values.contribution_amount, null); // blank
  assert.equal(r.get(9).values.contributor_first_name, "Ann");
  assert.equal(r.get(9).values.contribution_amount, null); // past the end of a short row
  assert.deepEqual(r.get(10).extraFields, ["x", "y", "z"]);
  assert.equal(Object.keys(r.get(10).values).length, 45);
  // TEXT rows: the first column is rec_type (Q16).
  assert.equal(r.get(11).rowType, "TEXT");
  assert.equal(r.get(11).values.rec_type, "TEXT");
  assert.ok(!("form_type" in r.get(11).values));
  assert.equal(r.get(15).rowType, "F3S");
  assert.equal(r.get(16).rowType, "F3PS");
  assert.equal(r.get(16).values.net_contributions, "125.50"); // a text column in the mapping
  assert.equal(r.get(16).values.a_i_individuals_itemized, 7);
  assert.equal(r.get(18).values.contributor_last_name, "Last");
});

test("dates: 'date' gives Date at UTC midnight in values; garbage stays null", () => {
  const r = byLine(drain("itemizationRecords", { dates: "date" }));
  assert.deepEqual(r.get(3).values.contribution_date, new Date("2025-07-07T00:00:00Z"));
  assert.equal(r.get(6).values.contribution_date, null);
  // The typed record keeps ISO strings.
  assert.equal(r.get(3).itemization.contribution_date, "2025-07-07");
});

test("itemizations(): types, Unknown for rows with no struct, garbage as the parser gives it", () => {
  const items = drain("itemizations");
  assert.deepEqual(
    items.map((x) => x.type),
    [
      "ScheduleA", "ScheduleA", "ScheduleA", "ScheduleA", "ScheduleA", "ScheduleA", "ScheduleA", "ScheduleA",
      "TextRecord", "ScheduleA3L", "ScheduleC", "ScheduleC1", "Unknown", "Unknown",
      "ScheduleA", "ScheduleA", "ScheduleA",
    ],
  );
  const at = (line) => items[line - 3];
  assert.equal(at(3).contributor.name.last_name, "Muñoz");
  assert.equal(at(4).contributor.name.last_name, "😀Smith");
  assert.equal(at(5).contributor.name.last_name, "O�Neil");
  // T3: garbage in the typed record is the parser's value; Record.values keeps the raw.
  assert.equal(at(7).contribution_amount, 0);
  assert.equal(at(6).contribution_date, null);
  assert.equal(at(11).text, "A note");
  assert.equal(at(12).bundled_amount, 10);
  assert.equal(at(17).contributor.name.last_name, 'O"Brien');
  assert.equal(at(18).contributor.name.last_name, "Last");
  assert.equal(at(19).contributor.name.last_name, '"A');
  // F3S (mapped, no struct) and F3PS (a summary record) come out as Unknown.
  for (const [line, formType] of [[15, "F3S"], [16, "F3PS"]]) {
    const u = at(line);
    assert.ok(u instanceof Unknown);
    assert.equal(u.formType, formType);
    assert.equal(u.record.rowType, formType);
    assert.equal(u.record.line, line);
    assert.deepEqual(u.toJSON(), { type: "Unknown", formType, values: u.record.values });
  }
  assert.equal(at(16).record.values.a_i_individuals_itemized, 7);
});

test("the two views are one-to-one, up to the same error", () => {
  const items = drain("itemizations");
  const records = drain("itemizationRecords");
  assert.equal(items.length, records.length);
  records.forEach((rec, i) => {
    const item = items[i];
    if (item instanceof Unknown) {
      assert.equal(item.record.line, rec.line);
      assert.equal(rec.itemization, null);
    } else {
      assert.equal(item.form_type, rec.rowType);
      assert.deepEqual(rec.itemization, item, `line ${rec.line}`);
    }
  });
});
