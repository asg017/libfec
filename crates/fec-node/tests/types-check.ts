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

// The Filing API.
import {
  Filing,
  MissingMappingError,
  Record,
  Unknown,
  type FilingOptions,
  type RecordJSON,
  type UnknownJSON,
  type Value,
} from "../js/index.js";

const filing = new Filing("x.fec", { dates: "date", id: "1", batchSize: 10 } satisfies FilingOptions);
filing.formType satisfies string;
filing.filerId satisfies string;
filing.filerName satisfies string;
filing.reportCode satisfies string | null;
filing.coverageFromDate satisfies string | Date | null;
filing.id satisfies string | null;
filing.sourceLength satisfies number;
filing.closed satisfies boolean;
new Filing(new Uint8Array(0));
new Filing(new URL("file:///x.fec"));
new Filing(new ArrayBuffer(0));
// @ts-expect-error: unknownRows was dropped
new Filing("x.fec", { unknownRows: "skip" });
// @ts-expect-error: not a dates mode
new Filing("x.fec", { dates: "Date" });

// `switch (item.type)` narrows Itemization | Unknown, "Unknown" included.
for (const item of filing.itemizations()) {
  switch (item.type) {
    case "ScheduleA":
      item.contribution_amount satisfies number;
      // @ts-expect-error: a ScheduleB field
      item.expenditure_amount;
      break;
    case "Unknown":
      item satisfies Unknown;
      item.formType satisfies string;
      item.record satisfies Record;
      item.toJSON() satisfies UnknownJSON;
      // @ts-expect-error: an Unknown has no typed fields
      item.contribution_amount;
      break;
  }
}
const items: Generator<Itemization | Unknown, void, undefined> = filing.itemizations();
void items;
// @ts-expect-error: no filter argument
filing.itemizations("ScheduleA");

for (const record of filing.itemizationRecords()) {
  record satisfies Record;
  record.rowType satisfies string;
  record.line satisfies number;
  record.fields satisfies readonly string[];
  record.values.contribution_amount satisfies Value | undefined;
  record.get("contribution_amount") satisfies Value | undefined;
  record.extraFields satisfies string[];
  record.toJSON() satisfies RecordJSON;
  // The lazy typed view: an Itemization, never an Unknown.
  const it = record.itemization;
  it satisfies Itemization | null;
  if (it?.type === "ScheduleB") it.expenditure_amount satisfies number;
  // @ts-expect-error: itemization is read-only
  record.itemization = null;
}
// @ts-expect-error: no filter argument
filing.itemizationRecords("SA");

// The cover narrows the same way.
const c = filing.cover;
switch (c.type) {
  case "Form3X":
    c.summary.line6c_total_receipts.column_a satisfies number;
    break;
  case "Unknown":
    c.record satisfies Record;
    break;
}
filing.coverRecord satisfies Record;
// @ts-expect-error: the cover is never null
filing.cover satisfies null;

// Importing `Record` shadows TypeScript's global `Record<K, V>` in this module.
// @ts-expect-error: here `Record` is the class, which takes no type arguments
type Shadowed = Record<string, number>;
const plain: { [k: string]: number } = { a: 1 };
void plain;

// Errors.
const mm = new MissingMappingError("ZZ9", "8.4", 3);
mm.rowType satisfies string;
mm.line satisfies number;

// fromId.
Filing.fromId(1721696) satisfies Promise<Filing>;
Filing.fromId("FEC-1721696", { proxy: "https://p.example/?url=", timeout: 1000, dates: "date" });
Filing.fromId(1, { proxy: (u: string) => u, signal: AbortSignal.timeout(10) });
// @ts-expect-error: the id is the first argument, not an option
Filing.fromId(1, { id: "1" });

filing.close();
filing[Symbol.dispose]();

// Removed names are gone.
// @ts-expect-error: open() was removed
import { open } from "../js/index.js";
// @ts-expect-error: read() was removed
import { read } from "../js/index.js";
// @ts-expect-error: FilingReader was removed
import type { FilingReader } from "../js/index.js";
// @ts-expect-error: Row was renamed Record
import type { Row } from "../js/index.js";
void [open, read];
// @ts-expect-error: no rows()
filing.rows();
// @ts-expect-error: no records()
filing.records();
// @ts-expect-error: no coverSummary
filing.coverSummary;
// @ts-expect-error: no coverRow
filing.coverRow;
// @ts-expect-error: no invalidValues
filing.invalidValues;
// @ts-expect-error: no skippedRows
filing.skippedRows;
// @ts-expect-error: not iterable itself
for (const _ of filing) void _;

// The RSS feed.
import { rss } from "../js/index.js";

const feed = rss.parseFeed("<rss/>");
for (const item of feed.items) {
  item.pubDate satisfies Date | null;
  item.coverageFrom satisfies string | null;
  item.committeeName satisfies string;
}
rss.feedUrl({ preset: "pac", forms: ["F3X"], states: "CA" }) satisfies string;
// @ts-expect-error: not a preset
rss.feedUrl({ preset: "weekly" });
rss.fetch({ signal: AbortSignal.timeout(1000) }) satisfies Promise<rss.Feed>;
rss.fetch({ proxy: "https://p.example/?url=" });
rss.fetch({ proxy: (u) => u });
