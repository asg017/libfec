// Read a whole filing, then group typed receipts by contributor city.
import { read } from "../dist/index.js";
import { PAC, arg } from "./_fixture.mjs";

const filing = read(arg(PAC));
// Narrow to ScheduleA records first, so TypeScript knows their fields.
const receipts = filing
  .itemizations()
  .map((row) => row.itemization)
  .filter((it) => it.type === "ScheduleA");
const byCity = Object.groupBy(receipts, (it) => it.contributor.address.city ?? "?");
const top = Object.entries(byCity)
  .map(([city, rows]) => /** @type {[string, number]} */ ([city, rows?.length ?? 0]))
  .sort((a, b) => b[1] - a[1])
  .slice(0, 3);
console.log(filing.rows.length, "rows;", receipts.length, "receipts; top cities:", top);
