// Iterator helpers on raw rows: the first 3 SA11AI receipts.
import { open } from "../dist/index.js";
import { PAC, arg } from "./_fixture.mjs";

const filing = open(arg(PAC));
try {
  const firstThree = filing
    .rows()
    .filter((row) => row.rowType === "SA11AI")
    .take(3)
    .map((row) => [row.values.contributor_last_name, row.values.contribution_amount])
    .toArray();
  console.log(firstThree);
} finally {
  filing.close();
}
