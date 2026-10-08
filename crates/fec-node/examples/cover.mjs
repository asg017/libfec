// The typed cover, matched on its form.
import { open } from "../dist/index.js";
import { PAC, arg } from "./_fixture.mjs";

const filing = open(arg(PAC));
filing.close(); // the header and cover stay readable after close()

const cover = filing.cover;
switch (cover?.type) {
  case "Form3X":
    console.log(cover.committee_name, "raised", cover.summary.line6c_total_receipts.column_a);
    break;
  case "Form3":
    console.log(cover.committee_name, "net contributions", cover.summary.line6c_net_contributions.column_a);
    break;
  case "Form99":
    console.log(cover.committee_name, "sent a miscellaneous text filing");
    break;
  default:
    console.log(filing.coverSummary.formType, "filed by", filing.coverSummary.filerName);
}
