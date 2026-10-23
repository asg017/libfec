// Count records per itemization type. In TypeScript, the `never` default makes
// the compiler flag a type added to the union later.
import { Filing } from "../dist/index.js";
import { PAC, arg } from "./_fixture.mjs";

const filing = new Filing(arg(PAC));
/** @type {{ [type: string]: number }} */
const counts = {};
try {
  for (const item of filing.itemizations()) {
    switch (item.type) {
      case "ScheduleA":
      case "ScheduleB":
      case "ScheduleC":
      case "ScheduleD":
      case "ScheduleE":
      case "Unknown": // a record with no typed class (a summary record, …)
        counts[item.type] = (counts[item.type] ?? 0) + 1;
        break;
      default:
        // TypeScript: `const _exhaustive: never = item;` once every case is listed.
        counts.other = (counts.other ?? 0) + 1;
    }
  }
} finally {
  filing.close();
}
console.log(counts);
