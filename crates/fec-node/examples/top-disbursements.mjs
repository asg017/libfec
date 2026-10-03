// The five largest disbursements (Schedule B) of a filing.
import { open } from "../dist/index.js";
import { PAC, arg } from "./_fixture.mjs";

const filing = open(arg(PAC));
/** @type {[number, string | null][]} */
const top = [];
try {
  for (const { itemization: it } of filing.itemizations()) {
    if (it.type !== "ScheduleB") continue;
    top.push([it.expenditure_amount, it.payee.organization_name ?? it.payee.name.last_name]);
    top.sort((a, b) => b[0] - a[0]);
    top.length = Math.min(top.length, 5);
  }
} finally {
  filing.close();
}
console.table(top);
