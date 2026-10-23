// The typed cover, matched on its form.
import { Filing } from "../dist/index.js";
import { PAC, arg } from "./_fixture.mjs";

const filing = new Filing(arg(PAC));
filing.close(); // the header, cover and filer attributes stay readable after close()

const cover = filing.cover;
switch (cover.type) {
  case "Form3X":
    console.log(cover.committee_name, "raised", cover.summary.line6c_total_receipts.column_a);
    break;
  case "Form3":
    console.log(cover.committee_name, "net contributions", cover.summary.line6c_net_contributions.column_a);
    break;
  case "Form99":
    console.log(cover.committee_name, "sent a miscellaneous text filing");
    break;
  case "Unknown":
    // A form with no typed struct: the raw cover line is still there.
    console.log(cover.formType, "has no typed cover;", Object.keys(cover.record.values).length, "columns");
    break;
  default:
    console.log(filing.formType, "filed by", filing.filerName);
}
