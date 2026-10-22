// The private native layer (NativeReader, schema): batches, modes, the cover.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import * as native from "../native/native.js";
import { TokenDecoder } from "../dist/tokens.js";
import { fixture } from "./helpers.mjs";

const SAMPLE = fixture("1921705.fec");
const PAC = fixture("1721696.fec");

/** Every row of a reader as { fields, line, item } (item: decoded, typed mode). */
function drain(reader, n = 1024) {
  const out = [];
  const dec = new TokenDecoder();
  for (let b; (b = reader.nextBatch(n)); ) {
    const items = b.typed ? dec.decode(b.typed) : null;
    if (b.typed) assert.equal(b.typed.rows, b.rowEnds.length);
    let f = 0;
    for (let i = 0; i < b.rowEnds.length; i++) {
      const fields = [];
      for (; f < b.rowEnds[i]; f++) {
        fields.push(b.text.slice(f === 0 ? 0 : b.ends[f - 1], b.ends[f]));
      }
      out.push({ fields, line: b.lines[i], item: items ? items[i] : null, typed: b.typed });
    }
  }
  return out;
}

test("raw batches: row types, lines, no tokens", () => {
  const rows = drain(native.NativeReader.openPath(SAMPLE));
  assert.deepEqual(
    rows.map((r) => r.fields[0]),
    ["SA11AI", ...Array(13).fill("SA11C"), "SA11D", ...Array(5).fill("SB17")],
  );
  assert.deepEqual(rows.map((r) => r.line), Array.from({ length: 20 }, (_, i) => i + 3));
  assert.ok(rows.every((r) => r.typed === null));
  assert.equal(drain(native.NativeReader.openPath(PAC)).length, 1387);
});

test("rows round-trip against a naive split of the file", () => {
  for (const name of ["1921705.fec", "1721696.fec", "1913562.fec", "1923816.fec"]) {
    const path = fixture(name);
    const naive = readFileSync(path, "latin1")
      .split(/\r?\n/)
      .filter((l) => l !== "")
      .slice(2)
      .map((l) => l.split("\x1c"));
    const rows = drain(native.NativeReader.openPath(path)).map((r) => r.fields);
    assert.deepEqual(rows, naive, name);
  }
});

test("batch size doesn't change the rows", () => {
  const a = drain(native.NativeReader.openPath(PAC), 1).map((r) => r.fields);
  const b = drain(native.NativeReader.openPath(PAC), 100000).map((r) => r.fields);
  assert.deepEqual(a, b);
});

test("typed mode: tokens for exactly the kept rows; untyped rows skipped", () => {
  const r = native.NativeReader.openPath(PAC);
  r.setTyped(true);
  const rows = drain(r);
  assert.equal(rows.length, 1387); // Python: every row of this file types
  const sa = rows.find((x) => x.fields[0] === "SA11AI").item;
  assert.equal(sa.type, "ScheduleA");
  assert.equal(sa.contributor.name.last_name, "Aaronson");
  assert.equal(sa.contribution_amount, 104.17);
  assert.equal(sa.form_type, "SA11AI");
  // 1913562 (F1A): 3 rows, none with a typed struct (Python agrees).
  const f1 = native.NativeReader.openPath(fixture("1913562.fec"));
  f1.setTyped(true);
  assert.equal(drain(f1).length, 0);
  assert.equal(drain(native.NativeReader.openPath(fixture("1913562.fec"))).length, 3);
});

test("struct ids are stable across batches (new structs listed once)", () => {
  const r = native.NativeReader.openPath(PAC);
  r.setTyped(true);
  const seen = new Set();
  for (let b; (b = r.nextBatch(50)); ) {
    for (const d of b.typed.newStructs) {
      const id = d.split("\x1f")[0];
      assert.ok(!seen.has(id), `struct ${id} announced twice`);
      seen.add(id);
    }
  }
  assert.ok(seen.size > 3);
});

test("setTyped after the first batch is refused", () => {
  const r = native.NativeReader.openPath(SAMPLE);
  r.nextBatch(1);
  assert.throws(() => r.setTyped(true), { code: "ERR_FILING_CONSUMED" });
});

test("cover: summary, raw fields, line, typed cover tokens", () => {
  const r = native.NativeReader.openPath(PAC);
  assert.equal(r.id, "1721696");
  assert.equal(r.fecVersion, "8.4");
  assert.deepEqual(r.coverSummary, {
    formType: "F3XN",
    filerId: "C00016683",
    filerName: "PFIZER INC. PAC",
    reportCode: "M8",
    coverageFromDate: "2023-07-01",
    coverageThroughDate: "2023-07-31",
  });
  assert.equal(r.coverFields.length, 123);
  assert.equal(r.coverFields[0], "F3XN");
  assert.equal(r.coverLine, 2);
  const [cover] = new TokenDecoder().decode(r.coverData());
  assert.equal(cover.type, "Form3X");
  assert.equal(cover.committee_name, "PFIZER INC. PAC");
  assert.equal(cover.summary.line6c_total_receipts.column_a, 83741.93);
  const [f99] = new TokenDecoder().decode(native.NativeReader.openPath(fixture("1913493.fec")).coverData());
  assert.equal(f99.type, "Form99");
});

test("end of file is sticky; close() is idempotent and stops reading", () => {
  const r = native.NativeReader.openPath(SAMPLE);
  drain(r);
  assert.equal(r.nextBatch(10), null);
  assert.equal(r.nextBatch(10), null);
  const c = native.NativeReader.openPath(SAMPLE);
  c.close();
  c.close();
  assert.equal(c.closed, true);
  assert.throws(() => c.nextBatch(1), { code: "ERR_FILING_CLOSED" });
});

test("bytes: same rows as the path; id is null", () => {
  const r = native.NativeReader.openBytes(new Uint8Array(readFileSync(SAMPLE)));
  assert.equal(r.id, null);
  assert.deepEqual(
    drain(r).map((x) => x.fields),
    drain(native.NativeReader.openPath(SAMPLE)).map((x) => x.fields),
  );
});

test("open errors carry a code", () => {
  assert.throws(() => native.NativeReader.openPath("nope.fec"), { code: "ENOENT" });
  assert.throws(() => native.NativeReader.openBytes(new TextEncoder().encode("garbage")), {
    code: "FEC_PARSE",
  });
});

test("schema: names and kinds; unmapped throws", () => {
  assert.deepEqual(native.schema("SA11AI", "8.4").slice(19, 21), [
    { name: "contribution_date", kind: "date" },
    { name: "contribution_amount", kind: "amount" },
  ]);
  assert.throws(() => native.schema("XX", "8.4"), { code: "FEC_MISSING_MAPPING" });
});
