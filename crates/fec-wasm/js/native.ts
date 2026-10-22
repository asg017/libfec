// The browser build's platform module: the wasm-bindgen glue (crates/fec-wasm)
// instead of the napi addon. scripts/build.mjs puts this file in place of
// crates/fec-node/js/native.ts; every other module in js/ is shared as is.
//
// The module initializes the wasm with top-level await, so once
// `import ... from "@alex.garcia/libfec-wasm"` resolves, everything is synchronous,
// as in Node.

import init, * as wasm from "../wasm/libfec.js";
import type { Batch, NativeCoverSummary, NativeHeader, TokenBatch } from "../wasm/libfec.js";

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
export { parseRssBytes, parseRssText, readHeaderPath, rssFeedUrl, schema, version } from "../wasm/libfec.js";

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

// ---- limits and traps ------------------------------------------------------

/**
 * The largest filing the browser build opens. The bytes are copied into wasm
 * memory, and a wasm32 allocation tops out just under 2 GiB; past that the
 * allocator traps with a bare `RuntimeError: unreachable`.
 */
export const MAX_BYTES = 2 ** 31 - 2 ** 24;

/** `readHeader` needs only the start of a file; don't copy the rest into wasm. */
const HEADER_BYTES = 2 ** 26;

function tooLarge(n: number): Error {
  const e = new RangeError(
    `the filing is ${n} bytes; the browser build reads at most ${MAX_BYTES} ` +
      "(wasm32 memory). Use @alex.garcia/libfec in Node, Deno or Bun for bigger filings",
  );
  return Object.assign(e, { code: "ERR_FILING_TOO_LARGE" });
}

/**
 * A trap (out of memory, or a Rust panic: `panic = "abort"`) surfaces as a
 * bare `WebAssembly.RuntimeError`. Give it a code and a message; the reader it
 * came from can't be used again.
 */
function trapped(e: unknown): unknown {
  if (!(e instanceof WebAssembly.RuntimeError)) return e;
  return Object.assign(
    new Error(`the WebAssembly parser stopped (${e.message}): out of memory, or a bug; this reader can't be used again`, {
      cause: e,
    }),
    { code: "ERR_WASM_TRAP" },
  );
}

export function readHeaderBytes(bytes: Uint8Array): NativeHeader {
  try {
    return wasm.readHeaderBytes(bytes.length > HEADER_BYTES ? bytes.subarray(0, HEADER_BYTES) : bytes);
  } catch (e) {
    throw trapped(e);
  }
}

/**
 * The wasm `NativeReader`, with the browser's limits: inputs over
 * {@link MAX_BYTES} are refused up front, and after a trap the reader is
 * marked dead, so `close()` (which runs in `finally` blocks) doesn't replace
 * the real error with wasm-bindgen's "recursive use of an object".
 */
export class NativeReader {
  #inner: wasm.NativeReader;
  #dead = false;

  private constructor(inner: wasm.NativeReader) {
    this.#inner = inner;
  }

  static openPath(path: string): NativeReader {
    return new NativeReader(wasm.NativeReader.openPath(path));
  }

  static openBytes(bytes: Uint8Array): NativeReader {
    if (bytes.length > MAX_BYTES) throw tooLarge(bytes.length);
    try {
      return new NativeReader(wasm.NativeReader.openBytes(bytes));
    } catch (e) {
      throw trapped(e);
    }
  }

  #call<T>(f: (r: wasm.NativeReader) => T): T {
    if (this.#dead) {
      throw Object.assign(new Error("this reader stopped after a WebAssembly trap"), { code: "ERR_WASM_TRAP" });
    }
    try {
      return f(this.#inner);
    } catch (e) {
      if (e instanceof WebAssembly.RuntimeError) this.#dead = true;
      throw trapped(e);
    }
  }

  get id(): string | null {
    return this.#call((r) => r.id);
  }
  get fecVersion(): string {
    return this.#call((r) => r.fecVersion);
  }
  get header(): NativeHeader {
    return this.#call((r) => r.header);
  }
  get coverSummary(): NativeCoverSummary {
    return this.#call((r) => r.coverSummary);
  }
  get coverFields(): string[] {
    return this.#call((r) => r.coverFields);
  }
  get coverLine(): number {
    return this.#call((r) => r.coverLine);
  }
  get closed(): boolean {
    return this.#dead || this.#call((r) => r.closed);
  }
  coverData(): TokenBatch {
    return this.#call((r) => r.coverData());
  }
  setTyped(typed: boolean): void {
    this.#call((r) => r.setTyped(typed));
  }
  nextBatch(n: number): Batch | null {
    return this.#call((r) => r.nextBatch(n));
  }
  /** Idempotent, and a no-op on a reader that trapped (its memory is lost until the page reloads). */
  close(): void {
    if (!this.#dead) this.#call((r) => r.close());
  }
}
