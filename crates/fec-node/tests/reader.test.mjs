import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import { FecError, FecParseError, MissingMappingError, Row, open } from "../dist/index.js";
import { fixture } from "./helpers.mjs";

const SAMPLE = fixture("1921705.fec");
const PAC = fixture("1721696.fec");
const F99 = fixture("1913493.fec");

function all(reader, method = "rows") {
  try {
    return [...reader[method]()];
  } finally {
    reader.close();
  }
}

/** `1921705.fec`'s HDR + cover, then `body`. */
function withBody(body) {
  const [hdr, cover] = readFileSync(SAMPLE, "latin1").split(/\r?\n/);
  return new TextEncoder().encode(`${hdr}\n${cover}\n${body}`);
}

test("sources: path, URL, bytes, ArrayBuffer give the same rows", () => {
  const want = all(open(SAMPLE)).map((r) => r.fields);
  assert.equal(want.length, 20);
  const bytes = readFileSync(SAMPLE);
  for (const src of [pathToFileURL(SAMPLE), new Uint8Array(bytes), bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.length)]) {
    assert.deepEqual(all(open(src)).map((r) => r.fields), want);
  }
  assert.equal(open(new Uint8Array(bytes)).id, null);
  assert.equal(open(SAMPLE).id, "1921705");
});

test("head: header, cover, coverSummary, coverRow before iterating and after close", () => {
  const f = open(PAC);
  const check = () => {
    assert.equal(f.fecVersion, "8.4");
    assert.equal(f.header.fecVersion, "8.4");
    assert.equal(f.coverSummary.formType, "F3XN");
    assert.equal(f.coverSummary.filerId, "C00016683");
    assert.equal(f.coverSummary.coverageFromDate, "2023-07-01");
    assert.equal(f.coverRow.values.col_a_total_receipts, 83741.93);
    assert.equal(Object.keys(f.coverRow.values).length, 123);
    assert.equal(f.coverRow.itemization, null);
    assert.ok(f.coverRow instanceof Row);
    assert.equal(f.cover.type, "Form3X");
    assert.ok(!("form" in f.cover) && !("data" in f.cover));
    assert.equal(f.cover.committee_name, "PFIZER INC. PAC");
    assert.equal(f.cover.summary.line6c_total_receipts.column_a, 83741.93);
  };
  check();
  f.close();
  check();
  assert.equal(open(F99).cover.type, "Form99");
  assert.equal(open(fixture("1923816.fec")).cover.type, "Form1");
  const d = open(PAC, { dates: "date" });
  assert.ok(d.coverSummary.coverageFromDate instanceof Date);
  assert.equal(d.coverSummary.coverageFromDate.toISOString(), "2023-07-01T00:00:00.000Z");
});

test("itemizations(): typed rows, type tags, no family key", () => {
  const rows = all(open(SAMPLE), "itemizations");
  assert.equal(rows.length, 20);
  assert.deepEqual(
    rows.map((r) => r.itemization.type),
    [...Array(15).fill("ScheduleA"), ...Array(5).fill("ScheduleB")],
  );
  for (const r of rows) {
    assert.equal(r.itemization.form_type, r.rowType);
    assert.ok(!("family" in r.itemization));
  }
  const pac = all(open(PAC), "itemizations");
  assert.equal(pac.length, 1387); // Python: every row of this filing types
  const sb = pac.filter((r) => r.rowType.startsWith("SB"));
  assert.equal(sb.length, 33);
  assert.ok(sb.every((r) => r.itemization.type === "ScheduleB"));
});

test("rows(): untyped", () => {
  const rows = all(open(PAC));
  assert.equal(rows.length, 1387);
  assert.ok(rows.every((r) => r.itemization === null));
});

test("records(): raw strings with lines", () => {
  const recs = all(open(SAMPLE), "records");
  assert.deepEqual(recs.map((r) => r.line), Array.from({ length: 20 }, (_, i) => i + 3));
  assert.ok(recs.every((r) => r.fields[0] === r.rowType));
});

test("one consuming call per reader, even after exhaustion", () => {
  const consumers = {
    itemizations: (f) => [...f.itemizations()],
    rows: (f) => [...f.rows()],
    records: (f) => [...f.records()],
    forOf: (f) => [...f],
  };
  for (const [a, first] of Object.entries(consumers)) {
    for (const [b, second] of Object.entries(consumers)) {
      const f = open(SAMPLE);
      first(f);
      assert.throws(() => second(f), { code: "ERR_FILING_CONSUMED" }, `${a} then ${b}`);
      f.close();
    }
  }
  const f = open(SAMPLE);
  for (const _ of f) break;
  assert.throws(() => f.itemizations(), (e) => e instanceof FecError && e.code === "ERR_FILING_CONSUMED");
  f.close();
});

test("filters were removed: passing one is a TypeError", () => {
  const f = open(SAMPLE);
  assert.throws(() => f.rows("SA"), { name: "TypeError", message: /filters were removed/ });
  assert.throws(() => f.itemizations("SA"), TypeError);
  assert.throws(() => f.records("SA"), TypeError);
  f.close();
});

test("close(): mid-iteration, idempotent, and new calls throw", () => {
  const f = open(PAC, { batchSize: 10 });
  const it = f.rows();
  it.next();
  f.close();
  f.close();
  assert.equal(f.closed, true);
  assert.throws(() => it.next(), { code: "ERR_FILING_CLOSED" });
  const g = open(SAMPLE);
  g.close();
  assert.throws(() => g.rows(), { code: "ERR_FILING_CLOSED" });
  const h = open(SAMPLE);
  h[Symbol.dispose]();
  assert.equal(h.closed, true);
});

test("unknownRows: throw (default), skip; itemizations() skips quietly", () => {
  const [, , real] = readFileSync(SAMPLE, "latin1").split(/\r?\n/);
  const bytes = withBody(`ZZ9\x1cC00000001\n${real}\n`);
  assert.throws(
    () => all(open(bytes)),
    (e) => e instanceof MissingMappingError && e.rowType === "ZZ9" && e.line === 3 && e.fecVersion === "8.5",
  );
  const skip = open(bytes, { unknownRows: "skip" });
  assert.equal(all(skip).length, 1);
  assert.deepEqual(skip.skippedRows, [{ rowType: "ZZ9", line: 3 }]);
  const typed = open(bytes);
  assert.equal(all(typed, "itemizations").length, 1);
  assert.deepEqual(typed.skippedRows, []);
});

test("invalidValues: garbage amounts and dates, from rows and the cover", () => {
  const [, , real] = readFileSync(SAMPLE, "latin1").split(/\r?\n/);
  const fields = real.split("\x1c");
  fields[19] = "20230230";
  fields[20] = "12abc";
  const f = open(withBody(fields.join("\x1c") + "\n"));
  const [row] = all(f, "itemizations");
  assert.equal(row.values.contribution_amount, "12abc");
  assert.equal(row.values.contribution_date, null);
  assert.equal(row.itemization.contribution_amount, 0);
  assert.equal(row.itemization.contribution_date, null);
  assert.deepEqual(f.invalidValues, [
    { line: 3, column: "contribution_date", raw: "20230230" },
    { line: 3, column: "contribution_amount", raw: "12abc" },
  ]);
  assert.equal(f.invalidValueCount, 2);
});

test("F99: zero rows", () => {
  assert.equal(all(open(F99)).length, 0);
  assert.equal(all(open(F99), "itemizations").length, 0);
});

test("iterator helpers work on the generators", () => {
  const f = open(PAC);
  const got = f.rows().filter((r) => r.rowType === "SA11AI").take(10).toArray();
  assert.equal(got.length, 10);
  f.close();
});

test("options are validated", () => {
  assert.throws(() => open(SAMPLE, { dates: "unix" }), TypeError);
  assert.throws(() => open(SAMPLE, { unknownRows: "warn" }), TypeError);
  assert.throws(() => open(SAMPLE, { batchSize: 0 }), TypeError);
  assert.equal(all(open(PAC, { batchSize: 1 })).length, 1387);
});

test("open errors: ENOENT, FecParseError", () => {
  assert.throws(() => open("nope.fec"), { code: "ENOENT", path: "nope.fec" });
  assert.throws(() => open(new TextEncoder().encode("garbage")), FecParseError);
});

test("id option: names a bytes source; overrides a path's", () => {
  const bytes = new Uint8Array(readFileSync(SAMPLE));
  assert.equal(open(bytes).id, null);
  assert.equal(open(bytes, { id: "1921705" }).id, "1921705");
  assert.equal(open(SAMPLE).id, "1921705");
  assert.equal(open(SAMPLE, { id: "custom" }).id, "custom");
  assert.throws(() => open(bytes, { id: 1921705 }), TypeError);
});
