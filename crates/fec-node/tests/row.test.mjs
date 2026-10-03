// Row building (js/row.ts) over native batches, and the value rules.
import { test } from "node:test";
import assert from "node:assert/strict";
import * as native from "../native/native.js";
import { Row } from "../dist/index.js";
import {
  Converters,
  InvalidValueSink,
  RowBuilder,
  parseAmount,
  parseDate,
} from "../dist/row.js";
import { TokenDecoder } from "../dist/tokens.js";
import { fixture } from "./helpers.mjs";

const SAMPLE = fixture("1921705.fec");
const PAC = fixture("1721696.fec");
const THROW = (rowType, line) => {
  throw new Error(`unmapped ${rowType} at ${line}`);
};

function rows(path, { typed = false, dates = "iso", codegen } = {}) {
  const r = native.NativeReader.openPath(path);
  r.setTyped(typed);
  const sink = new InvalidValueSink();
  const b = new RowBuilder(r.fecVersion, new Converters(sink, dates), { codegen });
  const dec = typed ? new TokenDecoder({ codegen }) : null;
  const out = [];
  for (let batch; (batch = r.nextBatch(1024)); ) out.push(...b.build(batch, dec, THROW));
  return { rows: out, sink };
}

test("values: typed by column kind", () => {
  const [first] = rows(SAMPLE).rows;
  assert.equal(first.values.contribution_amount, 500);
  assert.equal(first.values.contribution_date, "2025-07-07");
  const sa = rows(PAC).rows.find((r) => r.rowType === "SA11AI");
  assert.equal(sa.values.contributor_last_name, "Aaronson");
  assert.equal(sa.values.contribution_amount, 104.17);
  assert.equal(sa.values.contribution_aggregate, 1458.38);
  assert.equal(sa.values.contribution_date, "2023-07-14");
  assert.equal(sa.values.transaction_id, "2023071716378-1066");
  assert.deepEqual(Object.keys(sa.values), native.schema("SA11AI", "8.4").map((c) => c.name));
});

test("typed mode: itemization next to values", () => {
  const sa = rows(PAC, { typed: true }).rows.find((r) => r.rowType === "SA11AI");
  assert.equal(sa.itemization.type, "ScheduleA");
  assert.ok(!("family" in sa.itemization));
  assert.equal(sa.itemization.contributor.name.last_name, "Aaronson");
  assert.equal(sa.itemization.contribution_amount, 104.17);
  assert.equal(sa.itemization.contribution_date, "2023-07-14");
  assert.equal(sa.itemization.form_type, sa.rowType);
});

test("raw mode: itemization is null", () => {
  assert.ok(rows(PAC).rows.every((r) => r.itemization === null));
});

test("Row: own data properties, methods on the prototype, toJSON", () => {
  const [row] = rows(SAMPLE, { typed: true }).rows;
  assert.ok(row instanceof Row);
  assert.deepEqual(Object.keys(row), ["rowType", "line", "values", "itemization", "fields"]);
  assert.equal(row.rowType, "SA11AI");
  assert.equal(row.line, 3);
  assert.equal(row.fields[0], row.rowType);
  assert.equal(row.get("contribution_amount"), 500);
  assert.equal(row.get("nope"), undefined);
  assert.equal(row.get("toString"), undefined);
  assert.deepEqual(row.extraFields, []);
  assert.deepEqual(Object.keys(JSON.parse(JSON.stringify(row))), [
    "rowType", "line", "values", "itemization",
  ]);
  const clone = structuredClone(row);
  assert.equal(Object.getPrototypeOf(clone), Object.prototype);
  assert.deepEqual(clone.values, row.values);
});

test("codegen fallback gives identical rows", () => {
  for (const path of [SAMPLE, PAC]) {
    const a = rows(path, { typed: true, codegen: true }).rows;
    const b = rows(path, { typed: true, codegen: false }).rows;
    assert.equal(JSON.stringify(a), JSON.stringify(b));
    assert.deepEqual(a.map((r) => r.fields), b.map((r) => r.fields));
  }
});

test("dates: 'date' gives Date at UTC midnight, values only", () => {
  const [row] = rows(SAMPLE, { typed: true, dates: "date" }).rows;
  assert.ok(row.values.contribution_date instanceof Date);
  assert.equal(row.values.contribution_date.toISOString(), "2025-07-07T00:00:00.000Z");
  assert.equal(row.itemization.contribution_date, "2025-07-07");
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

test("date validity: 8 digits forming a real calendar date", () => {
  assert.deepEqual(parseDate("20240229"), [2024, 2, 29]);
  assert.deepEqual(parseDate("00000101"), [0, 1, 1]);
  for (const s of ["20230229", "20230230", "20231301", "20230100", "2023011", "+20230101", "2023-01-01", "202301011", "19000229"]) {
    assert.equal(parseDate(s), undefined, s);
  }
  assert.deepEqual(parseDate("20000229"), [2000, 2, 29]);
});

test("value rules: blanks, garbage, short rows, invalidValues", () => {
  const sink = new InvalidValueSink();
  const b = new RowBuilder("8.4", new Converters(sink, "iso"));
  const fields = native.schema("SA11AI", "8.4").map(() => "");
  fields[0] = "SA11AI";
  fields[7] = "  O'BRIEN ";
  fields[19] = "20230230";
  fields[20] = " 1,000.00 ";
  fields[21] = "   ";
  const row = b.row(fields, 9, null, THROW);
  assert.equal(row.values.contributor_last_name, "  O'BRIEN ");
  assert.equal(row.values.contributor_first_name, "");
  assert.equal(row.values.contribution_date, null);
  assert.equal(row.values.contribution_amount, " 1,000.00 ");
  assert.equal(row.values.contribution_aggregate, null);
  assert.deepEqual(sink.list, [
    { line: 9, column: "contribution_date", raw: "20230230" },
    { line: 9, column: "contribution_amount", raw: " 1,000.00 " },
  ]);
  const short = b.row(["SA11AI", "C001"], 10, null, THROW);
  assert.equal(short.values.contribution_amount, null);
  assert.equal(short.values.filer_committee_id_number, "C001");
  assert.equal(short.values.transaction_id, null);
  const long = b.row([...fields.slice(0, 45), "", "x"], 11, null, THROW);
  assert.deepEqual(long.extraFields, ["", "x"]);
  assert.deepEqual(b.row([...fields.slice(0, 45), "", ""], 12, null, THROW).extraFields, []);
});

test("invalidValues caps the list at 1,000 but counts everything", () => {
  const sink = new InvalidValueSink();
  for (let i = 0; i < 1500; i++) sink.push(i, "c", "x");
  assert.equal(sink.list.length, 1000);
  assert.equal(sink.count, 1500);
});

test("unmapped row types go to the callback", () => {
  const b = new RowBuilder("8.4", new Converters(new InvalidValueSink(), "iso"));
  const seen = [];
  assert.equal(b.row(["ZZ9", "x"], 4, null, (t, l) => (seen.push([t, l]), "skip")), null);
  assert.deepEqual(seen, [["ZZ9", 4]]);
});
