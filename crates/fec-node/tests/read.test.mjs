import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { Filing, MissingMappingError, columns, open, read } from "../dist/index.js";
import { fixture } from "./helpers.mjs";

const SAMPLE = fixture("1921705.fec");
const PAC = fixture("1721696.fec");

test("read(): every row, re-iterable", () => {
  const f = read(PAC);
  assert.ok(f instanceof Filing);
  assert.equal(f.rows.length, 1387);
  assert.equal(f.length, 1387);
  assert.equal([...f].length, 1387);
  assert.equal([...f].length, 1387);
  assert.ok(f.rows.every((r) => r.itemization === null));
});

test("read() rows equal open().rows()", () => {
  const reader = open(SAMPLE);
  const [first] = reader.rows();
  reader.close();
  assert.deepEqual(read(SAMPLE).rows[0], first);
});

test("read(): cover matches open()", () => {
  const reader = open(PAC);
  assert.deepEqual(read(PAC).cover, reader.cover);
  assert.deepEqual(read(PAC).coverSummary, reader.coverSummary);
  reader.close();
});

test("itemizations(): typed, cached, from paths and bytes", () => {
  const f = read(PAC);
  const items = f.itemizations();
  assert.equal(items, f.itemizations());
  assert.equal(items.length, 1387);
  assert.ok(items.every((r) => r.itemization !== null));
  const b = read(new Uint8Array(readFileSync(SAMPLE)));
  assert.equal(b.itemizations().length, 20);
  const byState = Object.groupBy(
    f.itemizations().filter((r) => r.itemization.type === "ScheduleA"),
    (r) => r.itemization.contributor.address.state,
  );
  assert.equal(byState.NY.length, 1354); // the PAC lists its employer's address for everyone
});

test("read(): a missing path is ENOENT", () => {
  assert.throws(() => read("nope.fec"), { code: "ENOENT" });
});

test("columns(): names and kinds; fresh arrays; unmapped throws", () => {
  assert.deepEqual(columns("SA11AI", "8.4").slice(19, 21), [
    { name: "contribution_date", kind: "date" },
    { name: "contribution_amount", kind: "amount" },
  ]);
  const a = columns("SA11AI", "8.4");
  a[0].name = "changed";
  assert.equal(columns("SA11AI", "8.4")[0].name, "form_type");
  assert.throws(
    () => columns("ZZ9", "8.4"),
    (e) => e instanceof MissingMappingError && e.line === 0 && e.rowType === "ZZ9",
  );
});

test("read(): the id option, and itemizations() keeps it", () => {
  const f = read(new Uint8Array(readFileSync(SAMPLE)), { id: "1921705" });
  assert.equal(f.id, "1921705");
  assert.equal(f.itemizations().length, 20);
  assert.equal(read(new Uint8Array(readFileSync(SAMPLE))).id, null);
});
