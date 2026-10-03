// Every itemization, matched on its type.
//   node examples/quickstart.mjs [file.fec]
import { open } from "../dist/index.js";
import { PAC, arg } from "./_fixture.mjs";

const filing = open(arg(PAC));
try {
  let receipts = 0;
  let disbursements = 0;
  for (const row of filing.itemizations()) {
    switch (row.itemization.type) {
      case "ScheduleA":
        receipts += row.itemization.contribution_amount;
        break;
      case "ScheduleB":
        disbursements += row.itemization.expenditure_amount;
        break;
    }
  }
  console.log(filing.coverSummary.filerName);
  console.log({ receipts: receipts.toFixed(2), disbursements: disbursements.toFixed(2) });
} finally {
  filing.close();
}
