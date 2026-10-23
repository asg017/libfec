# @alex.garcia/libfec-wasm

Parse FEC electronic filings (`.fec`) in the browser. This is libfec's Rust parser
compiled to WebAssembly, with **the same API as [`@alex.garcia/libfec`](../fec-node/README.md)**
for Node, Deno and Bun: `new Filing()` and `Filing.fromId()`, `itemizations()` and
`itemizationRecords()`, typed covers, `Record`, `Unknown`, `readHeader()`, label
functions, errors and `rss`. Only the sources differ: the browser has no file system, so
it reads bytes. The Node README documents the API, including the
[breaking changes](../fec-node/README.md#breaking-changes-the-filing-api) from
`open()`/`read()`; this one covers what is specific to the browser.

```ts
import { Filing } from "@alex.garcia/libfec-wasm";

const file = input.files[0]; // a <input type="file"> pick, or a drop
const filing = new Filing(new Uint8Array(await file.arrayBuffer()), { id: "1805248" });
try {
  console.log(filing.filerName, filing.formType);
  for (const item of filing.itemizations()) {
    switch (item.type) {
      case "ScheduleA": console.log(item.contributor.name.last_name, item.contribution_amount); break;
      case "ScheduleB": console.log(item.payee.organization_name, item.expenditure_amount); break;
      case "Unknown": break; // a record with no typed class
    }
  }
} finally {
  filing.close();
}
```

Or by filing ID, through a CORS proxy (see [below](#cors-filingfromid-and-the-rss-feed)):

```ts
const filing = await Filing.fromId(1721696, { proxy: "https://your-proxy.example/?url=" });
```

The [demo](demo/) is a complete app: drop a filing, or enter a filing ID or pick one from
the live feed (downloaded with `Filing.fromId` through the dev server's proxy), then parse
it in a Web Worker and explore it. Run it with `make demo`.

## Install

There are two ways to get it, both from the same build:

| | Package | When |
|---|---|---|
| A | `@alex.garcia/libfec-wasm` | browser-only apps |
| B | `@alex.garcia/libfec` | code that runs in both Node and the browser. Bundlers (Vite, webpack, esbuild, Rollup) pick this build through the `"browser"` export condition, and `@alex.garcia/libfec/browser` imports it explicitly |

Bundlers need no configuration. The module loads `libfec_bg.wasm` with
`new URL("../wasm/libfec_bg.wasm", import.meta.url)`, which Vite and webpack 5 recognize
and copy as an asset. A Vite 7 build of a one-file app produces a 17.5 KB (6.7 KB gzip)
JS bundle plus the wasm.

Without a bundler, serve the package directory and import `dist/index.js` from a
`<script type="module">`. The server must send `.wasm` as `application/wasm`.

**Loading:** the module initializes the wasm with top-level await. Once
`import "@alex.garcia/libfec-wasm"` resolves, every call is synchronous, as in Node.

## Differences from the Node build

| | Node/Deno/Bun (`@alex.garcia/libfec`) | Browser (`@alex.garcia/libfec-wasm`) |
|---|---|---|
| Sources | path, `file:` URL, `Uint8Array`, `ArrayBuffer` | `Uint8Array` or `ArrayBuffer`. A path or URL throws a `TypeError` with `code: "ERR_UNSUPPORTED_SOURCE"` |
| `File`/`Blob` | — | `new Uint8Array(await file.arrayBuffer())` |
| `filing.id` | from the file name, or `{ id }` | `null` unless you pass `{ id }` (`Filing.fromId` sets it) |
| Memory | the file is read in place | the bytes are **copied** into wasm memory once, when the `Filing` is constructed, so peak memory is about twice the file until you drop your copy. Every pass reads that one copy. The memory is reused by the next filing but never shrinks |
| Size limit | none | just under **2 GiB** (wasm32 memory). Bigger filings throw a `RangeError` with `code: "ERR_FILING_TOO_LARGE"`; use the Node build for those |
| Traps | — | if the parser runs out of memory or panics, the call throws an `Error` with `code: "ERR_WASM_TRAP"`. That filing can't be read again (its `close()` is safe to call), but the module stays usable |
| `Filing.fromId`, `rss.fetch` | work | need a `proxy`: the FEC sends no CORS headers (see below) |
| Everything else | | identical: same classes, values, typed records, errors, codes and messages |

"Identical" is tested. The test suite compares the two builds through the public API on
every committed fixture (157 filings, from v1 to v8.5 and paper), value for value and
error for error. It does so in Node, Deno and Bun, and in headless Chrome.

## Web Workers

Big filings belong in a worker: typing runs about 15 ms per MB on the main thread. Typed
itemizations and covers are plain data, so they survive `postMessage`. A `Record` or an
`Unknown` is cloned as a plain object without its methods (`record.itemization`,
`unknown.toJSON()`), so post `record.values` or `unknown.toJSON()` instead. One gotcha comes from the top-level await:
**set `onmessage` synchronously and import the library dynamically.** A message that
arrives while a module worker's top-level await is pending is dispatched before a
statically imported handler exists, and it's lost.

```js
// worker.mjs, started with new Worker(url, { type: "module" })
const lib = import("@alex.garcia/libfec-wasm");

self.onmessage = async ({ data: bytes }) => {
  const { Filing } = await lib;
  const filing = new Filing(bytes);
  try {
    const items = filing.itemizations().map((item) => (item.type === "Unknown" ? item.toJSON() : item));
    self.postMessage({ filer: filing.filerName, items: [...items] });
  } finally {
    filing.close();
  }
};
```

## CORS: `Filing.fromId` and the RSS feed

`rss.parseFeed()` and `rss.feedUrl()` work as in Node. `Filing.fromId()` and `rss.fetch()`
need a proxy in a browser, because `docquery.fec.gov` (where the `.fec` files are) and the
FEC's feed send no `Access-Control-Allow-Origin`:

```js
const PROXY = "https://your-proxy.example/?url=";
const feed = await rss.fetch({ preset: "pac", proxy: PROXY });
const filing = await Filing.fromId(feed.items[0].filingId, { proxy: PROXY });
```

`proxy` is a URL prefix, a template containing `{url}`, or a function. Without one, the
browser blocks the request, and both throw a `TypeError` with `code: "ERR_CORS"` that names
CORS as the likely cause (the original error is its `cause`). Through a proxy, a missing
filing is `FecError` `ERR_FILING_NOT_FOUND` and any other non-2xx status `ERR_HTTP`, as in
Node. `scripts/serve.mjs` has a small
FEC-only proxy for development (`/proxy?url=`). It only answers on localhost, only for
`efilingapps.fec.gov` and `docquery.fec.gov`, and re-checks every redirect.

## Content Security Policy

The wasm needs `'wasm-unsafe-eval'` in `script-src`. `'unsafe-eval'` is **not**
needed: where `new Function` is blocked, the row and record factories fall back to plain
loops with the same results. The browser tests run the page and its worker under
`script-src 'self' 'wasm-unsafe-eval'`.

## Performance

Measured on `1805248.fec` (91 MB, 408,160 rows), macOS arm64, best of 3:

Measured before the `Filing` reshape, with the passes that `itemizations()` and
`itemizationRecords()` replaced (`itemizations()` and `rows()`):

| | napi (Node 24) | wasm (Node 24) | wasm (Chrome 138) |
|---|---|---|---|
| `itemizations()` | 1.36 s | 1.63 s | 1.69 s |
| `itemizationRecords()` | 0.38 s | 0.40 s | 0.41 s |

Raw records run at about native speed. Typed records cost about 1.2× native, and nearly
all of that is the Rust typing, which runs slower as wasm than as native code. Size:
2.5 MB raw, **835 KB gzip, 511 KB brotli**.

Optimization choices: `[profile.wasm-release]` (fat LTO, one codegen unit, `opt-level = 3`,
`panic = "abort"`) and `wasm-opt -O3`. Tried and rejected:
- SIMD128: no gain, and it would cut off older browsers.
- The `talc` allocator: 5% slower than dlmalloc.
- `wasm-opt -O4`: more than 6 minutes per build.
- Larger batches: under 3%.

## How it's built

`crates/fec-wasm` is the wasm-bindgen twin of `crates/fec-node`'s napi layer. It has the
same `NativeSource`/`NativeReader`, `schema`, `typeRecord`, `readHeader*`, RSS functions and error codes, over the same
batch encodings (`crates/fec-js-core`). The TypeScript layer in `crates/fec-node/js` is
shared unchanged. `scripts/build.mjs` copies it into `build/src`, swaps in this package's
`js/native.ts` (the wasm loader instead of the napi addon), and compiles to `dist/`.

```
crates/fec-node/js/*.ts ──────┐  (shared: Filing/Record/Unknown/tokens/errors/rss/generated types)
crates/fec-wasm/js/native.ts ─┴─► tsc ─► dist/ ──────────────────────────┐
crates/fec-wasm/src ─► cargo + wasm-bindgen + wasm-opt ─► wasm/ ─────────┴─► @alex.garcia/libfec-wasm
                                                                           └─► @alex.garcia/libfec (browser/)
```

## Development

```bash
make install-tools   # wasm32 target, wasm-bindgen-cli 0.2.129 (pinned to Cargo.toml), wasm-opt reminder
make install         # npm ci (typescript, puppeteer-core)
make build           # optimized wasm + JS; `make build-dev` for fast unoptimized rebuilds
make test            # parity with the napi build + browser-only behavior, under node/deno/bun
                     # (needs `make build` in ../fec-node)
make test-browser    # headless Chrome (CHROME_PATH=...): parity, worker, CSP, rss, then the demo
make bench-browser   # the modes in headless Chrome on benchmarks/1805248.fec
make demo            # the demo on http://127.0.0.1:8000/fec-wasm/demo/
```

`node ../../benchmarks/node/bench.mjs --pkg wasm` benchmarks this build in Node, Deno and
Bun. In `crates/fec-node`, `make build-browser` copies this build into its `browser/`
directory for the `"browser"` condition.
