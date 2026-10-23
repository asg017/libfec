// Driven by tests/browser/run.mjs through puppeteer: window.__run(fixtures)
// resolves to what the wasm build says about each fixture, plus the results
// of the browser-only checks.
import * as lib from "../../dist/index.js";
// The same module instance dist/native.js initialized: init() hands back its
// exports, for the wasm memory.
import init from "../../wasm/libfec.js";
import { bigFiling, memoryPerPass } from "./memory.mjs";
import { summarize } from "./summarize.mjs";

const status = document.getElementById("status");

async function bytesOf(path) {
  const res = await fetch(path);
  if (!res.ok) throw new Error(`${path}: ${res.status}`);
  return new Uint8Array(await res.arrayBuffer());
}

function codegenAllowed() {
  try {
    return new Function("return 1")() === 1;
  } catch {
    return false;
  }
}

/** Parse one filing in a module worker; the rows come back by structured clone. */
function inWorker(data) {
  return new Promise((resolve, reject) => {
    const worker = new Worker(new URL("./worker.mjs", import.meta.url), { type: "module" });
    worker.onmessage = (e) => {
      worker.terminate();
      e.data.error ? reject(new Error(e.data.error)) : resolve(e.data);
    };
    worker.onerror = (e) => {
      worker.terminate();
      reject(new Error(e.message));
    };
    worker.postMessage(data, [data.buffer]);
  });
}

async function check(name, f) {
  try {
    return { name, ok: true, value: await f() };
  } catch (e) {
    return { name, ok: false, error: `${e?.name}: ${e?.message}` };
  }
}

window.__run = async (fixtures) => {
  const pac = "/fec-py/tests/fixtures/1721696.fec";
  // First, while the wasm heap is small: freed space from the fixtures below
  // could hide a copy.
  const memory = await check("memory per pass", async () => {
    const { memory } = await init();
    return memoryPerPass(lib, memory, bigFiling(await bytesOf(pac), 32).data);
  });
  const summaries = {};
  for (const path of fixtures) summaries[path] = summarize(lib, await bytesOf(path));

  const checks = [
    await check("codegen", () => codegenAllowed()),
    await check("version", () => lib.version),
    await check("a path is ERR_UNSUPPORTED_SOURCE", () => {
      try {
        new lib.Filing("1721696.fec");
        return "no error";
      } catch (e) {
        return `${e instanceof TypeError} ${e.code}`;
      }
    }),
    await check("File source via arrayBuffer()", async () => {
      const file = new File([await bytesOf(pac)], "1721696.fec");
      const f = new lib.Filing(new Uint8Array(await file.arrayBuffer()), { id: file.name.replace(/\.fec$/, "") });
      const n = [...f.itemizations()].length;
      const m = [...f.itemizationRecords()].length;
      f.close();
      return { id: f.id, n, m };
    }),
    await check("worker", async () => {
      const r = await inWorker(await bytesOf(pac));
      return { n: r.n, first: r.first, firstRecord: r.firstRecord, codegen: r.codegen };
    }),
    memory,
    await check("rss.fetch via proxy", async () => {
      const feed = await lib.rss.fetch({ preset: "pac", proxy: `${location.origin}/proxy?url=` });
      return { n: feed.items.length, first: feed.items[0].committeeName, pubDate: feed.items[0].pubDate instanceof Date };
    }),
    await check("rss.fetch without a proxy explains CORS", async () => {
      try {
        await lib.rss.fetch({ signal: AbortSignal.timeout(20_000) });
        return "no error";
      } catch (e) {
        return `${e.name}: ${/CORS/.test(e.message)}`;
      }
    }),
  ];
  status.textContent = "done";
  return { summaries, checks };
};
window.__ready = true;
status.textContent = "ready";
