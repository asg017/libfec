// Run the wasm build in headless Chrome and compare it with the napi build:
//
//   node tests/browser/run.mjs            # CHROME_PATH=... to pick the browser
//
// Serves crates/ (scripts/serve.mjs, with the saved RSS feed for the proxy),
// opens tests/browser/index.html twice (as is, and with every page and
// script, the worker's included, under a CSP without 'unsafe-eval', which
// turns off the codegen paths), and checks:
// - every committed fixture summarizes identically to the napi build in Node;
// - a module worker, a File source, ERR_UNSUPPORTED_SOURCE;
// - wasm memory doesn't grow by the file size per pass (memory.mjs);
// - rss.fetch through the proxy, and the CORS explanation without one.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import puppeteer from "puppeteer-core";
import { serve } from "../../scripts/serve.mjs";
import { FIXTURES, bytes, napi } from "../helpers.mjs";
import { memoryProblems } from "./memory.mjs";
import { summarize } from "./summarize.mjs";
import { findChrome } from "./chrome.mjs";

const CRATES = fileURLToPath(new URL("../../../", import.meta.url));
const fixtures = FIXTURES.map(([label]) => `/${label.replaceAll("\\", "/")}`);
const expected = Object.fromEntries(FIXTURES.map(([label, path]) => [`/${label.replaceAll("\\", "/")}`, summarize(napi, bytes(path))]));
// Matching the napi build means nothing if both just throw (an API that's
// gone is a TypeError): every fixture must summarize without one.
for (const [path, summary] of Object.entries(expected)) {
  assert.notEqual(summary.error?.name, "TypeError", `${path}: ${summary.error?.message}`);
  assert.ok(summary.error || summary.itemizationRecords, `${path}: no records`);
}

const FEED = fileURLToPath(new URL("../../../fec-rss/tests/fixtures/all.xml", import.meta.url));
const browser = await puppeteer.launch({ executablePath: findChrome(), headless: true, args: ["--no-sandbox"], protocolTimeout: 300_000 });
let failed = 0;
try {
  for (const csp of [false, true]) {
    const label = csp ? "strict CSP" : "default";
    const server = await serve({ root: CRATES, feed: FEED, csp: csp ? "strict" : undefined });
    const page = await browser.newPage();
    const errors = [];
    page.on("pageerror", (e) => errors.push(String(e)));
    const t0 = performance.now();
    // No network: the FEC's feed URL fails like a CORS block would (a TypeError
    // from fetch), so the "explains CORS" check doesn't depend on the FEC.
    // Only fec.gov is intercepted (puppeteer's page-wide interception would
    // also pause the worker's script).
    const cdp = await page.createCDPSession();
    cdp.on("Fetch.requestPaused", (e) => cdp.send("Fetch.failRequest", { requestId: e.requestId, errorReason: "AccessDenied" }));
    await cdp.send("Fetch.enable", { patterns: [{ urlPattern: "*://*.fec.gov/*" }] });
    await page.goto(`${server.url}/fec-wasm/tests/browser/index.html`);
    await page.waitForFunction(() => window.__ready === true, { timeout: 30_000 });
    const { summaries, checks } = await page.evaluate((f) => window.__run(f), fixtures);
    const seconds = ((performance.now() - t0) / 1000).toFixed(1);
    assert.deepEqual(errors, [], `${label}: page errors`);

    let same = 0;
    for (const path of fixtures) {
      try {
        // JSON round trip on our side too: the page's results came through it.
        assert.deepEqual(summaries[path], JSON.parse(JSON.stringify(expected[path])), path);
        same++;
      } catch (e) {
        failed++;
        console.error(`FAIL ${label} ${path}\n${String(e.message).slice(0, 2000)}`);
      }
    }
    console.log(`${label}: ${same}/${fixtures.length} fixtures match the napi build (${seconds}s)`);

    const byName = Object.fromEntries(checks.map((c) => [c.name, c]));
    const expectChecks = {
      codegen: !csp,
      version: napi.version,
      "a path is ERR_UNSUPPORTED_SOURCE": "true ERR_UNSUPPORTED_SOURCE",
      "File source via arrayBuffer()": { id: "1721696", n: 1387, m: 1387 },
      "rss.fetch via proxy": { n: 30, first: "ALLIED UNIVERSAL PAC", pubDate: true },
      "rss.fetch without a proxy explains CORS": "TypeError: true",
    };
    for (const [name, want] of Object.entries(expectChecks)) {
      const c = byName[name];
      try {
        assert.ok(c.ok, c.error);
        assert.deepEqual(c.value, want);
        console.log(`ok   ${label}: ${name}`);
      } catch (e) {
        failed++;
        console.error(`FAIL ${label}: ${name}: ${e.message}`);
      }
    }
    const mem = byName["memory per pass"];
    try {
      assert.ok(mem.ok, mem.error);
      assert.deepEqual(memoryProblems(mem.value), [], JSON.stringify(mem.value));
      console.log(`ok   ${label}: memory per pass`);
    } catch (e) {
      failed++;
      console.error(`FAIL ${label}: memory per pass: ${e.message}`);
    }
    const worker = byName.worker;
    try {
      assert.ok(worker.ok, worker.error);
      const f = new napi.Filing(bytes(FIXTURES.find(([l]) => l.endsWith("fec-py/tests/fixtures/1721696.fec"))[1]));
      const [first] = f.itemizations();
      const [firstRecord] = f.itemizationRecords();
      f.close();
      assert.equal(worker.value.n, 1387);
      assert.equal(worker.value.codegen, !csp, "the worker runs under the page's CSP mode");
      assert.deepEqual(worker.value.first, JSON.parse(JSON.stringify(first)));
      // A structured clone keeps the own properties (fields included), not toJSON()'s shape.
      assert.deepEqual(worker.value.firstRecord, JSON.parse(JSON.stringify({ ...firstRecord })));
      console.log(`ok   ${label}: worker`);
    } catch (e) {
      failed++;
      console.error(`FAIL ${label}: worker: ${e.message}`);
    }
    await page.close();
    await server.close();
  }
} finally {
  await browser.close();
}
if (failed) {
  console.error(`${failed} failure(s)`);
  process.exit(1);
}
