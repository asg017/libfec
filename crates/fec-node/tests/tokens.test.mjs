// The token decoder (js/tokens.ts) against serde_json's output of the same
// typed values, on every fixture. Needs a debug addon (`make build`):
// `debugTypedPath` is compiled out of release builds.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync } from "node:fs";
import * as native from "../native/native.js";
import { TokenDecoder } from "../dist/tokens.js";
import { FIXTURES, LEGACY_FIXTURES, fixture } from "./helpers.mjs";

const FILES = [
  ...readdirSync(FIXTURES).filter((f) => f.endsWith(".fec")).map((f) => fixture(f)),
  ...readdirSync(LEGACY_FIXTURES)
    .filter((f) => f.endsWith(".fec"))
    .map((f) => fixture(f, LEGACY_FIXTURES)),
];

// serde's `family` tag -> the struct name the JS layer puts in `type`.
const FAMILY_TYPE = {
  SA: "ScheduleA", SB: "ScheduleB", SD: "ScheduleD", SF: "ScheduleF",
  H1: "ScheduleH1", H2: "ScheduleH2", H3: "ScheduleH3", H4: "ScheduleH4",
  H5: "ScheduleH5", H6: "ScheduleH6", F56: "Form5Contribution", F57: "Form5Expenditure",
  F65: "Form6Contribution", F76: "Form7Communication", F91: "Form9ControllingPerson",
  F92: "Form9Donation", F93: "Form9Disbursement", F94: "Form9Candidate",
  F132: "Form13Donation", F133: "Form13Refund", SL: "ScheduleL", TEXT: "TextRecord",
  SA3L: "ScheduleA3L", SE: "ScheduleE", SC: "ScheduleC", SC1: "ScheduleC1", SC2: "ScheduleC2",
};

/** serde_json's itemization, with the documented JS reshape applied. */
function jsItemization(v) {
  if (v === null) return null;
  const { family, ...rest } = v;
  assert.ok(FAMILY_TYPE[family], `unknown family ${family}`);
  return { type: FAMILY_TYPE[family], ...rest };
}

const jsCover = (v) => (v === null ? null : { type: v.form, ...v.data });

// JSON.stringify compares key order too, which deepStrictEqual doesn't.
const same = (actual, expected, msg) =>
  assert.equal(JSON.stringify(actual), JSON.stringify(expected), msg);

const hasDebug = typeof native.debugTypedPath === "function";
const skip = hasDebug ? false : "release addon: debugTypedPath is debug-only";

for (const codegen of [true, false]) {
  test(`decoded typed values equal serde_json (codegen: ${codegen})`, { skip }, () => {
    let typed = 0;
    for (const path of FILES) {
      const dbg = native.debugTypedPath(path);
      const dec = new TokenDecoder({ codegen });
      const [cover] = dec.decode(dbg.cover);
      same(cover, jsCover(JSON.parse(dbg.coverJson)), `${path}: cover`);
      const rows = dec.decode(dbg.rows);
      assert.equal(rows.length, dbg.rowsJson.length, path);
      rows.forEach((row, i) => {
        same(row, jsItemization(JSON.parse(dbg.rowsJson[i])), `${path}: row ${i}`);
        if (row !== null) typed++;
      });
    }
    assert.ok(typed > 1000, `only ${typed} typed rows across the fixtures`);
  });
}

test("no serde tag reaches JS", { skip }, () => {
  const dbg = native.debugTypedPath(fixture("1721696.fec"));
  const dec = new TokenDecoder();
  const [cover] = dec.decode(dbg.cover);
  assert.equal(cover.type, "Form3X");
  assert.equal(Object.keys(cover)[0], "type");
  assert.ok(!("form" in cover) && !("data" in cover));
  const rows = dec.decode(dbg.rows).filter(Boolean);
  for (const r of rows) {
    assert.equal(Object.keys(r)[0], "type");
    assert.ok(!("family" in r));
  }
  assert.ok(rows.some((r) => r.type === "ScheduleA"));
  // Nested structs keep their plain keys.
  const sa = rows.find((r) => r.type === "ScheduleA");
  assert.deepEqual(Object.keys(sa.contributor), ["entity_type", "organization_name", "name", "address"]);
});

test("a struct id the decoder never registered is an error, not garbage", { skip }, () => {
  const dbg = native.debugTypedPath(fixture("1721696.fec"));
  assert.throws(
    () => new TokenDecoder().decode({ ...dbg.rows, newStructs: [] }),
    /unknown struct id/,
  );
});
