// The Filing class: sources, options, the up-front data, passes, close().
// Value-level checks are in parity.test.mjs (against Python), edge.test.mjs
// and record.test.mjs.
import { test } from "node:test";
import assert from "node:assert/strict";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { Filing, Record, Unknown, FecError, FecParseError, MissingMappingError } from "../dist/index.js";
import { fixture, LEGACY_FIXTURES } from "./helpers.mjs";

const PAC = fixture("1721696.fec");
/** 31 typed rows and 2 `SI` rows with no typed class. */
const MIXED = fixture("3.00_13801.fec", LEGACY_FIXTURES);
const NODE_FIXTURES = new URL("./fixtures/", import.meta.url);
const EDGE = fixture("edge.fec", NODE_FIXTURES);
const UNKNOWN_COVER = fixture("unknown-cover.fec", NODE_FIXTURES);

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

test("a row with no column mapping throws MissingMappingError in both views, then the pass is done", () => {
  const f = new Filing(EDGE);
  for (const view of ["itemizations", "itemizationRecords"]) {
    const it = f[view]();
    const seen = [];
    assert.throws(
      () => {
        for (const x of it) seen.push(x);
      },
      (e) => e instanceof MissingMappingError && e instanceof FecError && e.code === "FEC_MISSING_MAPPING" && e.rowType === 'C"' && e.line === 20,
      view,
    );
    // The rows before it were yielded; after it, nothing.
    assert.equal(seen.length, 17, view);
    assert.deepEqual(it.next(), { value: undefined, done: true });
    // A new pass starts over and stops at the same row.
    assert.throws(() => [...f[view]()], { rowType: 'C"', line: 20 });
  }
  f.close();
});

test("a relative path is the file it named at construction, after chdir too", () => {
  const cwd = process.cwd();
  const dir = mkdtempSync(join(tmpdir(), "libfec-"));
  try {
    mkdirSync(join(dir, "a"));
    mkdirSync(join(dir, "b"));
    copyFileSync(PAC, join(dir, "a", "FEC-1721696.fec"));
    // The same relative path under the next working directory: another filing.
    copyFileSync(EDGE, join(dir, "b", "FEC-1721696.fec"));
    process.chdir(join(dir, "a"));
    const f = new Filing("FEC-1721696.fec");
    assert.equal(f.id, "1721696");
    process.chdir(join(dir, "b"));
    assert.equal([...f.itemizations()].length, 1387);
    assert.equal([...f.itemizationRecords()].length, 1387);
    // A later pass that can't reopen it names the path as given, like the constructor.
    // Node on Windows prints the resolved path, as its own fs does: resolved
    // when the Filing was made, so it names the file in a/, not b/.
    const nodeOnWindows = process.platform === "win32" && !("Deno" in globalThis) && !("Bun" in globalThis);
    const shown = nodeOnWindows ? join(dir, "a", "FEC-1721696.fec") : "FEC-1721696.fec";
    rmSync(join(dir, "a", "FEC-1721696.fec"));
    assert.throws(() => f.itemizations().next(), {
      code: "ENOENT",
      syscall: "open",
      path: "FEC-1721696.fec",
      message: `ENOENT: no such file or directory, open '${shown}'`,
    });
    f.close();
  } finally {
    process.chdir(cwd);
    rmSync(dir, { recursive: true, force: true });
  }
});

test("ids: the file stem with FEC- stripped, or the id option", () => {
  const dir = mkdtempSync(join(tmpdir(), "libfec-"));
  try {
    for (const [name, id] of [["FEC-1721696.fec", "1721696"], ["1721696.fec", "1721696"], ["report.FEC", "report"]]) {
      const path = join(dir, name);
      copyFileSync(PAC, path);
      assert.equal(new Filing(path).id, id, name);
      assert.equal(new Filing(pathToFileURL(path)).id, id, `${name} (file: URL)`);
    }
    assert.equal(new Filing(join(dir, "FEC-1721696.fec"), { id: "override" }).id, "override");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("filer attributes: dates option, F99, F1", () => {
  const d = new Filing(PAC, { dates: "date" });
  assert.deepEqual(d.coverageFromDate, new Date("2023-07-01T00:00:00Z"));
  assert.deepEqual(d.coverageThroughDate, new Date("2023-07-31T00:00:00Z"));
  assert.ok(d.coverRecord.values.coverage_from_date instanceof Date);
  // Typed records keep ISO strings.
  assert.equal(d.cover.coverage_from_date, "2023-07-01");
  const f99 = new Filing(fixture("1913493.fec"));
  assert.equal(f99.formType, "F99");
  assert.equal(f99.cover.type, "Form99");
  const f1 = new Filing(fixture("1913562.fec"));
  assert.equal(f1.formType, "F1A");
  assert.equal(f1.cover.type, "Form1");
  assert.equal(f1.reportCode, null);
  assert.equal(f1.coverageFromDate, null);
  assert.equal(f1.coverageThroughDate, null);
});

test("options: unknown keys and bad values are TypeErrors", () => {
  for (const options of [null, 1, { batchsize: 10 }, { batchSize: 0 }, { batchSize: 1.5 }, { id: 5 }, { dates: "Date" }]) {
    assert.throws(() => new Filing(PAC, options), TypeError, JSON.stringify(options));
  }
  // batchSize doesn't change the results.
  const a = [...new Filing(PAC, { batchSize: 1 }).itemizationRecords()].map((r) => r.toJSON());
  const b = [...new Filing(PAC, { batchSize: 100000 }).itemizationRecords()].map((r) => r.toJSON());
  assert.deepEqual(a, b);
});

test("a cover with no typed struct is an Unknown", () => {
  // tests/fixtures/unknown-cover.fec: an F3PS line where the cover goes.
  const f = new Filing(UNKNOWN_COVER);
  assert.ok(f.cover instanceof Unknown);
  assert.equal(f.cover, f.cover);
  assert.equal(f.cover.type, "Unknown");
  assert.equal(f.cover.formType, "F3PS");
  assert.equal(f.cover.record, f.coverRecord);
  assert.equal(f.coverRecord.rowType, "F3PS");
  assert.equal(f.coverRecord.line, 2);
  assert.equal(f.cover.toJSON().values.net_contributions, "10.00");
  assert.equal(f.formType, "F3PS");
  assert.equal(f.filerId, "C00900860");
  assert.deepEqual([...f.itemizations()].map((x) => x.type), ["ScheduleA", "TextRecord"]);
  assert.deepEqual([...f.itemizationRecords()].map((r) => [r.rowType, r.line]), [["SA11AI", 3], ["TEXT", 4]]);
  f.close();
  // The same from bytes made here: any mapped row with no cover struct.
  const hdr = readFileSync(PAC, "latin1").split("\n")[0];
  const g = new Filing(new TextEncoder().encode(`${hdr}\nF1S\x1cC00016683\n`));
  assert.ok(g.cover instanceof Unknown);
  assert.equal(g.cover.formType, "F1S");
  assert.equal(g.formType, "F1S");
  assert.deepEqual([...g.itemizations()], []);
});

test("Unknown for summary records: F1S, F3PS", () => {
  const f1 = new Filing(fixture("1913562.fec"));
  const items = [...f1.itemizations()];
  const recs = [...f1.itemizationRecords()];
  const f1s = items.filter((x) => x.type === "Unknown");
  assert.ok(f1s.length > 0);
  for (const u of f1s) {
    assert.equal(u.formType, "F1S");
    assert.equal(u.record.rowType, "F1S");
    assert.equal(recs[items.indexOf(u)].line, u.record.line);
  }
  const edge = new Filing(EDGE);
  const it = edge.itemizations();
  const f3ps = [];
  try {
    for (const x of it) if (x.type === "Unknown" && x.formType === "F3PS") f3ps.push(x);
  } catch (e) {
    assert.ok(e instanceof MissingMappingError);
  }
  assert.equal(f3ps.length, 1);
  assert.equal(f3ps[0].record.rowType, "F3PS");
  assert.equal(f3ps[0].record.line, 16);
  assert.equal(f3ps[0].record.values.a_i_individuals_itemized, 7);
});

test("one-to-one: itemizations() and itemizationRecords() zip on line", () => {
  for (const path of [PAC, MIXED, fixture("1913562.fec"), fixture("1921705.fec")]) {
    const f = new Filing(path);
    const items = f.itemizations();
    const recs = f.itemizationRecords();
    let n = 0;
    for (;;) {
      const a = items.next();
      const b = recs.next();
      assert.equal(a.done, b.done, path);
      if (a.done) break;
      n++;
      const typedLine = a.value.type === "Unknown" ? a.value.record.line : b.value.line;
      assert.equal(typedLine, b.value.line);
      // A typed item has no line; its record is the one at the same step.
      if (a.value.type !== "Unknown") assert.deepEqual(b.value.itemization, a.value);
    }
    assert.ok(n > 0, path);
    f.close();
  }
});

test("passes: two full passes are equal; two interleaved step by step are equal", () => {
  for (const view of ["itemizations", "itemizationRecords"]) {
    const f = new Filing(PAC, { batchSize: 64 });
    const json = (x) => JSON.parse(JSON.stringify(x));
    const first = [...f[view]()].map(json);
    const second = [...f[view]()].map(json);
    assert.equal(first.length, 1387);
    assert.deepEqual(second, first);
    // Interleaved, one a step behind the other.
    const a = f[view]();
    const b = f[view]();
    const fromA = [];
    const fromB = [];
    let x = a.next();
    for (;;) {
      const y = b.next();
      if (!x.done) fromA.push(json(x.value));
      if (y.done) break;
      fromB.push(json(y.value));
      x = a.next();
    }
    assert.deepEqual(fromA, first);
    assert.deepEqual(fromB, first);
    // An abandoned pass doesn't disturb a new one.
    const c = f[view]();
    c.next();
    c.return();
    assert.deepEqual([...f[view]()].map(json), first);
    f.close();
  }
});
