// @network: download a filing from docquery.fec.gov by its ID. `make examples`
// skips it; `make examples-network` runs it.
//   node examples/from-id.mjs [filing-id]
import { Filing } from "../dist/index.js";
import { arg } from "./_fixture.mjs";

const id = arg("1721696"); // "1721696", "FEC-1721696" or the number 1721696
let filing;
try {
  filing = await Filing.fromId(id);
} catch (e) {
  // ERR_FILING_NOT_FOUND for a 404, ERR_HTTP for any other non-2xx status.
  const err = /** @type {{ code?: string, status?: number }} */ (e);
  if (err.code === "ERR_FILING_NOT_FOUND" || err.code === "ERR_HTTP") {
    console.error(`filing ${id}: ${err.code} (HTTP ${err.status})`);
    process.exit(1);
  }
  throw e;
}
try {
  console.log(filing.id, filing.formType, filing.filerName, `${filing.sourceLength} bytes`);
  let n = 0;
  for (const _ of filing.itemizationRecords()) n++;
  console.log(n, "itemization records");
} finally {
  filing.close();
}
