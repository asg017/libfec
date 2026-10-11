// Run the wasm build in headless Chrome and compare it with the napi build:
//
//   node tests/browser/run.mjs            # CHROME_PATH=... to pick the browser
//
// Serves crates/ (scripts/serve.mjs, with the saved RSS feed for the proxy),
// opens tests/browser/index.html twice (as is, and under a CSP without
// 'unsafe-eval', which turns off the codegen paths), and checks:
// - every committed fixture summarizes identically to the napi build in Node;
// - a module worker, a File source, ERR_UNSUPPORTED_SOURCE;
// - rss.fetch through the proxy, and the CORS explanation without one.
import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import puppeteer from "puppeteer-core";
import { serve } from "../../scripts/serve.mjs";
import { FIXTURES, bytes, napi } from "../helpers.mjs";
import { summarize } from "./summarize.mjs";

function findChrome() {
  if (process.env.CHROME_PATH) return process.env.CHROME_PATH;
  const candidates = [
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Chromium.app/Contents/MacOS/Chromium",
    "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
  ];
  for (const c of candidates) if (existsSync(c)) return c;
  for (const name of ["google-chrome-stable", "google-chrome", "chromium", "chromium-browser"]) {
    const r = spawnSync("which", [name], { encoding: "utf8" });
    if (r.status === 0) return r.stdout.trim();
  }
  throw new Error("no Chrome found; set CHROME_PATH");
}

const CRATES = fileURLToPath(new URL("../../../", import.meta.url));
const fixtures = FIXTURES.map(([label]) => `/${label.replaceAll("\\", "/")}`);
const expected = Object.fromEntries(FIXTURES.map(([label, path]) => [`/${label.replaceAll("\\", "/")}`, summarize(napi, bytes(path))]));

const server = await serve({ root: CRATES, feed: fileURLToPath(new URL("../../../fec-rss/tests/fixtures/all.xml", import.meta.url)) });
const browser = await puppeteer.launch({ executablePath: findChrome(), headless: true, args: ["--no-sandbox"], protocolTimeout: 300_000 });
let failed = 0;
try {
  for (const csp of [false, true]) {
    const label = csp ? "strict CSP" : "default";
    const page = await browser.newPage();
    const errors = [];
    page.on("pageerror", (e) => errors.push(String(e)));
    const t0 = performance.now();
    await page.goto(`${server.url}/fec-wasm/tests/browser/index.html${csp ? "?csp=strict" : ""}`);
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
      "File source via arrayBuffer()": { id: "1721696", n: 1387 },
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
    const worker = byName.worker;
    try {
      assert.ok(worker.ok, worker.error);
      const f = napi.open(bytes(FIXTURES.find(([l]) => l.endsWith("fec-py/tests/fixtures/1721696.fec"))[1]));
      const [first] = f.itemizations();
      f.close();
      assert.equal(worker.value.n, 1387);
      // A structured clone keeps the own properties (fields included), not toJSON()'s shape.
      assert.deepEqual(worker.value.first, JSON.parse(JSON.stringify({ ...first })));
      console.log(`ok   ${label}: worker`);
    } catch (e) {
      failed++;
      console.error(`FAIL ${label}: worker: ${e.message}`);
    }
    await page.close();
  }
} finally {
  await browser.close();
  await server.close();
}
if (failed) {
  console.error(`${failed} failure(s)`);
  process.exit(1);
}
