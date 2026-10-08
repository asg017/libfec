// Count rows per itemization type. In TypeScript, the `never` default makes
// the compiler flag a type added to the union later.
import { open } from "../dist/index.js";
import { PAC, arg } from "./_fixture.mjs";

const filing = open(arg(PAC));
/** @type {Record<string, number>} */
const counts = {};
try {
  for (const { itemization: it } of filing.itemizations()) {
    switch (it.type) {
      case "ScheduleA":
      case "ScheduleB":
      case "ScheduleC":
      case "ScheduleD":
      case "ScheduleE":
        counts[it.type] = (counts[it.type] ?? 0) + 1;
        break;
      default:
        // TypeScript: `const _exhaustive: never = it;` once every case is listed.
        counts.other = (counts.other ?? 0) + 1;
    }
  }
} finally {
  filing.close();
}
console.log(counts);
