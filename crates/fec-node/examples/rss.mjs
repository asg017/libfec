// The newest filings in the e-filing RSS feed, grouped by form type.
//   node examples/rss.mjs [feed.xml | --live]
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { rss } from "../dist/index.js";
import { arg } from "./_fixture.mjs";

const SAVED = fileURLToPath(new URL("../../fec-rss/tests/fixtures/all.xml", import.meta.url));

const source = arg(SAVED);
const feed = source === "--live" ? await rss.fetch() : rss.parseFeed(readFileSync(source));

/** @type {Map<string, number>} */
const byForm = new Map();
for (const item of feed.items) {
  const form = item.formType ?? "?";
  byForm.set(form, (byForm.get(form) ?? 0) + 1);
}
console.log(feed.title, `(${feed.items.length} items)`);
console.log(Object.fromEntries([...byForm].sort((a, b) => b[1] - a[1])));
const [newest] = feed.items;
if (newest) console.log(newest.pubDate?.toISOString(), newest.committeeName, newest.link);
