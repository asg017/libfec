// The five largest disbursements (Schedule B) of a filing.
import { Filing } from "../dist/index.js";
import { PAC, arg } from "./_fixture.mjs";

const filing = new Filing(arg(PAC));
/** @type {[number, string | null][]} */
const top = [];
try {
  for (const item of filing.itemizations()) {
    if (item.type !== "ScheduleB") continue;
    top.push([item.expenditure_amount, item.payee.organization_name ?? item.payee.name.last_name]);
    top.sort((a, b) => b[0] - a[0]);
    top.length = Math.min(top.length, 5);
  }
} finally {
  filing.close();
}
console.table(top);
