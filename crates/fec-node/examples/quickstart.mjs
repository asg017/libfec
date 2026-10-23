// Every itemization, matched on its type.
//   node examples/quickstart.mjs [file.fec]
import { Filing } from "../dist/index.js";
import { PAC, arg } from "./_fixture.mjs";

// `using filing = new Filing(…)` closes it on Node 24+, Deno and Bun; Node 22
// doesn't parse `using`, so the examples use try/finally.
const filing = new Filing(arg(PAC));
try {
  let receipts = 0;
  let disbursements = 0;
  for (const item of filing.itemizations()) {
    switch (item.type) {
      case "ScheduleA":
        receipts += item.contribution_amount;
        break;
      case "ScheduleB":
        disbursements += item.expenditure_amount;
        break;
    }
  }
  console.log(filing.filerName, filing.formType, filing.coverageFromDate, filing.coverageThroughDate);
  console.log({ receipts: receipts.toFixed(2), disbursements: disbursements.toFixed(2) });
} finally {
  filing.close();
}
