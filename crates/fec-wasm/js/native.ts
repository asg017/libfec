// The browser build's platform module: the wasm-bindgen glue (crates/fec-wasm)
// instead of the napi addon. scripts/build.mjs puts this file in place of
// crates/fec-node/js/native.ts; every other module in js/ is shared as is.
//
// The module initializes the wasm with top-level await, so once
// `import ... from "@asg017/libfec-wasm"` resolves, everything is synchronous,
// as in Node.

import init from "../wasm/libfec.js";

export type {
  Batch,
  Column,
  NativeCoverSummary,
  NativeHeader,
  NativeHeaderField,
  NativeRssFeed,
  NativeRssItem,
  TokenBatch,
} from "../wasm/libfec.js";
export {
  NativeReader,
  parseRssBytes,
  parseRssText,
  readHeaderBytes,
  readHeaderPath,
  rssFeedUrl,
  schema,
  version,
} from "../wasm/libfec.js";

// `new URL(..., import.meta.url)`, written out, so bundlers (Vite, webpack,
// esbuild with a loader) see the asset and copy it.
const WASM_URL = new URL("../wasm/libfec_bg.wasm", import.meta.url);

await init({
  module_or_path: WASM_URL.protocol === "file:" ? await readFileUrl(WASM_URL) : WASM_URL,
});

/**
 * Node, Deno and Bun importing the browser build from disk (tests, SSR):
 * Node's `fetch()` can't read `file:` URLs. The specifier is a variable so
 * bundlers leave this branch alone; browsers never reach it.
 */
async function readFileUrl(url: URL): Promise<Uint8Array> {
  const specifier = "node:fs/promises";
  const fs = (await import(/* @vite-ignore */ /* webpackIgnore: true */ specifier)) as {
    readFile(url: URL): Promise<Uint8Array>;
  };
  return fs.readFile(url);
}

/** A `file:` URL's path. Only reaches `openPath`, which throws `ERR_UNSUPPORTED_SOURCE`. */
export function urlToPath(url: URL): string {
  return decodeURIComponent(url.pathname);
}

/** Paths are never opened here, so I/O errors never need shaping. */
export function messagePath(path: string): string {
  return path;
}

/** No `errno` table in the browser. */
export function errnoOf(_code: string): number | undefined {
  return undefined;
}
