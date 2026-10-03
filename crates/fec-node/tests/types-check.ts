// Compile-only checks of the generated types (`make typecheck`); never run.
import type { Cover, Itemization, ScheduleA } from "../js/index.js";

declare const it: Itemization;
declare const cover: Cover;

// `type` narrows the union.
switch (it.type) {
  case "ScheduleA": {
    const amount: number = it.contribution_amount;
    const last: string = it.contributor.name.last_name;
    const date: string | null = it.contribution_date;
    // @ts-expect-error: a ScheduleB field isn't on ScheduleA
    it.expenditure_amount;
    void [amount, last, date];
    break;
  }
  case "ScheduleB":
    it.expenditure_amount satisfies number;
    break;
  case "TextRecord":
    it.text satisfies string | null;
    break;
}

// Exhaustiveness: every `type` is handled, so `default` sees `never`.
function exhaustive(i: Itemization): string {
  switch (i.type) {
    case "ScheduleA": case "ScheduleB": case "ScheduleD": case "ScheduleF":
    case "ScheduleH1": case "ScheduleH2": case "ScheduleH3": case "ScheduleH4":
    case "ScheduleH5": case "ScheduleH6": case "Form5Contribution": case "Form5Expenditure":
    case "Form6Contribution": case "Form7Communication": case "Form9ControllingPerson":
    case "Form9Donation": case "Form9Disbursement": case "Form9Candidate":
    case "Form13Donation": case "Form13Refund": case "ScheduleL": case "TextRecord":
    case "ScheduleA3L": case "ScheduleE": case "ScheduleC": case "ScheduleC1": case "ScheduleC2":
      return i.type;
    default: {
      const never: never = i;
      return never;
    }
  }
}
void exhaustive;

if (cover.type === "Form3X") {
  cover.summary.line6c_total_receipts.column_a satisfies number;
}

// No serde tags leak into the JS types.
// @ts-expect-error: no `family` key
it.family;
// @ts-expect-error: no `data` key on a flattened cover
cover.data;

type SA = Extract<Itemization, { type: "ScheduleA" }>;
const _sa: SA["contribution_amount"] = 1;
const _plain: ScheduleA["form_type"] = "SA11AI";
void [_sa, _plain];
