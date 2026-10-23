// The private native layer (NativeSource, NativeReader, schema, typeRecord):
// batches, modes, the cover.
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

test("typed mode: one token per row, null for untyped rows", () => {
  const rows = drain(native.NativeSource.openPath(PAC).reader(true));
  assert.equal(rows.length, 1387); // Python: every row of this file types
  assert.ok(rows.every((r) => r.item !== null));
  const sa = rows.find((x) => x.fields[0] === "SA11AI").item;
  assert.equal(sa.type, "ScheduleA");
  assert.equal(sa.contributor.name.last_name, "Aaronson");
  assert.equal(sa.contribution_amount, 104.17);
  assert.equal(sa.form_type, "SA11AI");
  // 1913562 (F1A): 3 F1S rows, none with a typed struct (Python agrees):
  // they are kept, with a null token each.
  const f1 = drain(native.NativeSource.openPath(fixture("1913562.fec")).reader(true), 2);
  assert.deepEqual(
    f1.map((r) => [r.fields[0], r.item]),
    [["F1S", null], ["F1S", null], ["F1S", null]],
  );
  assert.deepEqual(
    f1.map((r) => r.fields),
    drain(native.NativeReader.openPath(fixture("1913562.fec"))).map((r) => r.fields),
  );
});

test("struct ids are stable across batches (new structs listed once)", () => {
  const r = native.NativeSource.openPath(PAC).reader(true);
  assert.equal(r.typed, true);
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

test("NativeSource: many readers, identical passes, interleaved", () => {
  for (const src of [
    native.NativeSource.openPath(PAC),
    native.NativeSource.openBytes(new Uint8Array(readFileSync(PAC))),
  ]) {
    const a = drain(src.reader(false));
    const b = drain(src.reader(false));
    assert.equal(a.length, 1387);
    assert.deepEqual(a.map((r) => r.fields), b.map((r) => r.fields));
    // Two passes in lockstep don't disturb each other.
    const raw = src.reader(false);
    const typed = src.reader(true);
    assert.equal(raw.typed, false);
    let n = 0;
    for (let x, y; (x = raw.nextBatch(100)), (y = typed.nextBatch(100)), x || y; ) {
      assert.deepEqual(x.rowEnds, y.rowEnds);
      assert.equal(x.text, y.text);
      assert.equal(x.typed, null);
      assert.equal(y.typed.rows, y.rowEnds.length);
      n += x.rowEnds.length;
    }
    assert.equal(n, 1387);
    // Each reader has the header and cover.
    assert.equal(src.reader(true).coverSummary.filerId, "C00016683");
  }
});

test("NativeSource: id, sourceLength, close", () => {
  const p = native.NativeSource.openPath(PAC);
  assert.equal(p.id, "1721696");
  assert.equal(p.sourceLength, readFileSync(PAC).length);
  const bytes = new Uint8Array(readFileSync(SAMPLE));
  const b = native.NativeSource.openBytes(bytes);
  assert.equal(b.id, null);
  assert.equal(b.sourceLength, bytes.length);
  // A reader made before close() keeps reading; new ones are refused.
  const r = b.reader(false);
  assert.equal(b.closed, false);
  b.close();
  b.close();
  assert.equal(b.closed, true);
  assert.throws(() => b.reader(false), { code: "ERR_FILING_CLOSED" });
  assert.equal(drain(r).length, 20);
  assert.throws(() => native.NativeSource.openPath("nope.fec"), { code: "ENOENT" });
  // Bytes are parsed by reader(), not by openBytes.
  const g = native.NativeSource.openBytes(new TextEncoder().encode("garbage"));
  assert.throws(() => g.reader(false), { code: "FEC_PARSE" });
});

test("typeRecord: one row, the same value the typed batch has", () => {
  const rows = drain(native.NativeSource.openPath(PAC).reader(true));
  for (const r of [rows[0], rows.find((x) => x.fields[0] === "SB23")]) {
    const batch = native.typeRecord(r.fields, "8.4", null);
    assert.equal(batch.rows, 1);
    assert.deepEqual(new TokenDecoder().decode(batch), [r.item]);
  }
  const f1s = readFileSync(fixture("1913562.fec"), "latin1").split(/\r?\n/)[2].split("\x1c");
  assert.equal(f1s[0], "F1S");
  assert.deepEqual(new TokenDecoder().decode(native.typeRecord(f1s, "8.4", null)), [null]);
  assert.deepEqual(new TokenDecoder().decode(native.typeRecord(["XX"], "8.4", null)), [null]);
  assert.deepEqual(new TokenDecoder().decode(native.typeRecord([], "8.4", null)), [null]);
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
