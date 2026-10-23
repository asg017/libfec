// Collect a filing's typed receipts, then group them by contributor city.
import { Filing } from "../dist/index.js";
import { PAC, arg } from "./_fixture.mjs";

const filing = new Filing(arg(PAC));
let receipts;
try {
  // Narrow to ScheduleA first, so TypeScript knows the fields.
  receipts = filing
    .itemizations()
    .filter((item) => item.type === "ScheduleA")
    .toArray();
} finally {
  filing.close();
}
const byCity = Object.groupBy(receipts, (it) => it.contributor.address.city ?? "?");
const top = Object.entries(byCity)
  .map(([city, items]) => /** @type {[string, number]} */ ([city, items?.length ?? 0]))
  .sort((a, b) => b[1] - a[1])
  .slice(0, 3);
console.log(receipts.length, "receipts; top cities:", top);
