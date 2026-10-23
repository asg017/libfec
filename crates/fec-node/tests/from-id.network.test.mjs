// Live: hits docquery.fec.gov. Only under `make test-network`
// (`node scripts/test.mjs <runtime> --network`), never in `make test`.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { Filing, FecError } from "../dist/index.js";
import { fixture } from "./helpers.mjs";

test("fromId: live download of 1721696 matches the committed fixture", async () => {
  const filing = await Filing.fromId(1721696);
  try {
    assert.equal(filing.id, "1721696");
    assert.equal(filing.filerName, "PFIZER INC. PAC");
    assert.equal(filing.sourceLength, readFileSync(fixture("1721696.fec")).length);
    let n = 0;
    for (const _ of filing.itemizations()) n++;
    assert.equal(n, 1387);
  } finally {
    filing.close();
  }
});

test("fromId: live 404", async () => {
  await assert.rejects(Filing.fromId(99999999999), (e) => e instanceof FecError && e.code === "ERR_FILING_NOT_FOUND" && e.status === 404);
});
