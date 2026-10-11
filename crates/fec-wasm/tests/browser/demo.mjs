// End to end: the demo app (demo/) in headless Chrome, offline. The feed
// comes from the saved all.xml through the proxy, and a fixture is uploaded
// through the file input. Checks the feed list, the parse in the worker, the
// summary, paging and search.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import puppeteer from "puppeteer-core";
import { serve } from "../../scripts/serve.mjs";
import { findChrome } from "./chrome.mjs";

const FIXTURE = fileURLToPath(new URL("../../../fec-py/tests/fixtures/1721696.fec", import.meta.url));
const FEED = fileURLToPath(new URL("../../../fec-rss/tests/fixtures/all.xml", import.meta.url));

const server = await serve({ feed: FEED });
const browser = await puppeteer.launch({ executablePath: findChrome(), headless: true, args: ["--no-sandbox"] });
try {
  const page = await browser.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  await page.goto(`${server.url}/fec-wasm/demo/`);
  const text = (sel) => page.$eval(sel, (e) => e.textContent);
  await page.waitForFunction(() => !document.getElementById("feed-status").textContent.startsWith("Loading"));
  assert.match(await text("#feed-status"), /^30 filings/);
  assert.equal(await page.$$eval("#feed li", (l) => l.length), 30);

  const input = await page.$("#file");
  await input.uploadFile(FIXTURE);
  const done = () => page.waitForFunction(() => /^Parsed|\S/.test(document.getElementById("status").textContent) && !document.getElementById("status").textContent.startsWith("Parsing"));
  await done();
  assert.match(await text("#status"), /^Parsed 1,387 itemizations/);
  assert.equal(await text("#filer-name"), "PFIZER INC. PAC");
  assert.match(await text("#filer-meta"), /F3XN · M8 · C00016683 · 2023-07-01 to 2023-07-31 · FEC 8\.4/);
  assert.match(await text("#kpis"), /\$420,364Cash on hand/);
  assert.equal(await text("#page-info"), "1–50 of 1,354");

  await page.click("#next");
  await page.waitForFunction(() => document.getElementById("page-info").textContent.startsWith("51"));
  await page.type("#browse-q", "pfizer");
  await page.waitForFunction(() => document.getElementById("page-info").textContent === "1–50 of 1,255");
  await page.$eval("#browse-q", (e) => {
    e.value = "";
    e.dispatchEvent(new Event("input"));
  });
  await page.select("#browse-type", "ScheduleB");
  await page.waitForFunction(() => document.getElementById("page-info").textContent === "1–33 of 33");
  assert.deepEqual(errors, []);
  console.log("ok   demo: feed, upload, worker parse, summary, paging, search");
} finally {
  await browser.close();
  await server.close();
}
