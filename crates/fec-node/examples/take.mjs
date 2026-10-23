// Iterator helpers on raw records: the first 3 SA11AI receipts.
import { Filing } from "../dist/index.js";
import { PAC, arg } from "./_fixture.mjs";

const filing = new Filing(arg(PAC));
try {
  const firstThree = filing
    .itemizationRecords()
    .filter((record) => record.rowType === "SA11AI")
    .take(3)
    .map((record) => [record.values.contributor_last_name, record.values.contribution_amount])
    .toArray();
  console.log(firstThree);
} finally {
  filing.close();
}
