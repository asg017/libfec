// The browser build in headless Chrome on one filing, best of N per mode:
//
//   node tests/browser/bench.mjs [file] [--runs 3]
//
// Default file: benchmarks/1805248.fec (91 MB, local only). Compare with
// `node benchmarks/node/bench.mjs --pkg wasm|node` for Node numbers.
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import puppeteer from "puppeteer-core";
import { serve } from "../../scripts/serve.mjs";
import { findChrome } from "./chrome.mjs";

const args = process.argv.slice(2);
const runsAt = args.indexOf("--runs");
const runs = runsAt < 0 ? 3 : Number(args.splice(runsAt, 2)[1]);
const file = args[0] ?? fileURLToPath(new URL("../../../../benchmarks/1805248.fec", import.meta.url));
if (!existsSync(file)) {
  console.error(`no such file: ${file}`);
  process.exit(2);
}

const server = await serve({ files: { "/bench.fec": file } });
const browser = await puppeteer.launch({ executablePath: findChrome(), headless: true, protocolTimeout: 600_000 });
try {
  const page = await browser.newPage();
  await page.goto(`${server.url}/fec-wasm/tests/browser/bench.html`);
  await page.waitForFunction(() => window.__ready === true);
  const version = await browser.version();
  const r = await page.evaluate((runs) => window.__bench("/bench.fec", runs), runs);
  console.log(`\n${version} — wasm build — ${file} (${r.mb.toFixed(0)} MB, best of ${runs})\n`);
  console.log("| scenario | rows | seconds |\n|---|---:|---:|");
  for (const name of ["itemizations", "rows", "records", "read"]) {
    console.log(`| ${name} | ${r[name].rows} | ${r[name].seconds.toFixed(2)} |`);
  }
} finally {
  await browser.close();
  await server.close();
}
