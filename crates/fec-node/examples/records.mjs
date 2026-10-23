// The raw view: itemizationRecords(), with values by column name, the raw
// fields by position, and record.itemization to type one on demand.
import { Filing } from "../dist/index.js";
import { PAC, arg } from "./_fixture.mjs";

const filing = new Filing(arg(PAC));
try {
  for (const record of filing.itemizationRecords()) {
    if (record.rowType !== "SA11AI") continue;
    const at = Object.keys(record.values).indexOf("contribution_amount");
    console.log(record.line, record.rowType, record.values.contribution_amount, JSON.stringify(record.fields[at]));
    const item = record.itemization; // typed anew on every access
    if (item?.type === "ScheduleA") console.log(item.contributor.name.last_name, item.contribution_date);
    break;
  }
  console.log(filing.coverRecord.rowType, Object.keys(filing.coverRecord.values).length, "cover columns");
} finally {
  filing.close();
}
