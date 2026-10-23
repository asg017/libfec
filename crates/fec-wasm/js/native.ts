// The browser build's platform module: the wasm-bindgen glue (crates/fec-wasm)
// instead of the napi addon. scripts/build.mjs puts this file in place of
// crates/fec-node/js/native.ts; every other module in js/ is shared as is.
//
// Importing this module doesn't load the wasm: the entry points do (js/index.ts
// with `await init()`, js/workerd.ts with `initSync()`), or the caller does
// (js/manual.ts). Once it is loaded everything is synchronous, as in Node.

import initGlue, * as wasm from "../wasm/libfec.js";
import type { Batch, InitInput, NativeCoverSummary, NativeHeader, SyncInitInput, TokenBatch } from "../wasm/libfec.js";
import { VERSION } from "./version.js";

export type {
  Batch,
  Column,
  InitInput,
  NativeCoverSummary,
  NativeHeader,
  NativeHeaderField,
  NativeRssFeed,
  NativeRssItem,
  SyncInitInput,
  TokenBatch,
} from "../wasm/libfec.js";

// ---- loading the wasm ------------------------------------------------------

let ready = false;

/**
 * Loads the wasm from `input` (a URL, `Response`, bytes or a compiled
 * `WebAssembly.Module`), by default `libfec_bg.wasm` next to this package's
 * `wasm/libfec.js`. A no-op once the wasm is loaded.
 */
export async function init(input?: InitInput | Promise<InitInput>): Promise<void> {
  if (ready) return;
  if (input === undefined) {
    // `new URL(..., import.meta.url)`, written out, so bundlers (Vite, webpack,
    // esbuild with a loader) see the asset and copy it.
    const url = new URL("../wasm/libfec_bg.wasm", import.meta.url);
    input = url.protocol === "file:" ? await readFileUrl(url) : url;
  }
  await initGlue({ module_or_path: input });
  ready = true;
}

/**
 * Loads the wasm synchronously from its bytes or a compiled
 * `WebAssembly.Module`: in Cloudflare Workers, where wasm can't be compiled
 * at run time, `import wasm from "@alex.garcia/libfec-wasm/libfec_bg.wasm"`
 * gives a module to pass here. A no-op once the wasm is loaded.
 */
export function initSync(input: SyncInitInput): void {
  if (ready) return;
  wasm.initSync({ module: input });
  ready = true;
}

/** Throws `ERR_NOT_INITIALIZED` before {@link init} or {@link initSync}. */
function need(): void {
  if (ready) return;
  throw Object.assign(
    new Error(
      "the libfec wasm isn't loaded: call init() or initSync() from @alex.garcia/libfec-wasm/manual first, " +
        "or import @alex.garcia/libfec-wasm, which loads it",
    ),
    { code: "ERR_NOT_INITIALIZED" },
  );
}

/** The crate version, recorded at build time so reading it needs no wasm. */
export function version(): string {
  return VERSION;
}

export const parseRssBytes: typeof wasm.parseRssBytes = (...a) => (need(), wasm.parseRssBytes(...a));
export const parseRssText: typeof wasm.parseRssText = (...a) => (need(), wasm.parseRssText(...a));
export const readHeaderPath: typeof wasm.readHeaderPath = (...a) => (need(), wasm.readHeaderPath(...a));
export const rssFeedUrl: typeof wasm.rssFeedUrl = (...a) => (need(), wasm.rssFeedUrl(...a));
export const schema: typeof wasm.schema = (...a) => (need(), wasm.schema(...a));

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
  need();
  try {
    return wasm.readHeaderBytes(bytes.length > HEADER_BYTES ? bytes.subarray(0, HEADER_BYTES) : bytes);
  } catch (e) {
    throw trapped(e);
  }
}

/**
 * A cover settled further in than this is reported as `null`, which only
 * means "keep reading": the caller opens the whole file at its end.
 */
const COVER_BYTES = HEADER_BYTES;

export function coverEnd(prefix: Uint8Array): number | undefined {
  need();
  try {
    return wasm.coverEnd(prefix.length > COVER_BYTES ? prefix.subarray(0, COVER_BYTES) : prefix);
  } catch (e) {
    throw trapped(e);
  }
}

export function typeRecord(fields: string[], fecVersion: string, nameDelimiter: string | null): TokenBatch {
  need();
  try {
    return wasm.typeRecord(fields, fecVersion, nameDelimiter ?? undefined);
  } catch (e) {
    throw trapped(e);
  }
}

/** Runs `f` on a wasm object, marking `dead` after a trap. */
function guarded<O, T>(self: { inner: O; dead: boolean }, f: (o: O) => T): T {
  if (self.dead) {
    throw Object.assign(new Error("this reader stopped after a WebAssembly trap"), { code: "ERR_WASM_TRAP" });
  }
  try {
    return f(self.inner);
  } catch (e) {
    if (e instanceof WebAssembly.RuntimeError) self.dead = true;
    throw trapped(e);
  }
}

/**
 * The wasm `NativeSource`: the bytes, copied into wasm memory once, for any
 * number of {@link NativeReader} passes. Inputs over {@link MAX_BYTES} are
 * refused up front; after a trap it is marked dead, as a reader is.
 */
export class NativeSource {
  readonly #s: { inner: wasm.NativeSource; dead: boolean };

  private constructor(inner: wasm.NativeSource) {
    this.#s = { inner, dead: false };
  }

  static openPath(path: string): NativeSource {
    need();
    return new NativeSource(wasm.NativeSource.openPath(path));
  }

  static openBytes(bytes: Uint8Array): NativeSource {
    need();
    if (bytes.length > MAX_BYTES) throw tooLarge(bytes.length);
    try {
      return new NativeSource(wasm.NativeSource.openBytes(bytes));
    } catch (e) {
      throw trapped(e);
    }
  }

  reader(typed: boolean): NativeReader {
    return NativeReader.wrap(guarded(this.#s, (s) => s.reader(typed)));
  }
  get id(): string | null {
    return guarded(this.#s, (s) => s.id);
  }
  get sourceLength(): number {
    return guarded(this.#s, (s) => s.sourceLength);
  }
  get closed(): boolean {
    return this.#s.dead || guarded(this.#s, (s) => s.closed);
  }
  /** Idempotent, and a no-op after a trap. */
  close(): void {
    if (!this.#s.dead) guarded(this.#s, (s) => s.close());
  }
}

/**
 * The wasm `NativeReader`, with the browser's limits: inputs over
 * {@link MAX_BYTES} are refused up front, and after a trap the reader is
 * marked dead, so `close()` (which runs in `finally` blocks) doesn't replace
 * the real error with wasm-bindgen's "recursive use of an object".
 */
export class NativeReader {
  readonly #r: { inner: wasm.NativeReader; dead: boolean };

  private constructor(inner: wasm.NativeReader) {
    this.#r = { inner, dead: false };
  }

  /** @internal A reader made by {@link NativeSource.reader}. */
  static wrap(inner: wasm.NativeReader): NativeReader {
    return new NativeReader(inner);
  }

  static openPath(path: string): NativeReader {
    need();
    return new NativeReader(wasm.NativeReader.openPath(path));
  }

  static openBytes(bytes: Uint8Array): NativeReader {
    need();
    if (bytes.length > MAX_BYTES) throw tooLarge(bytes.length);
    try {
      return new NativeReader(wasm.NativeReader.openBytes(bytes));
    } catch (e) {
      throw trapped(e);
    }
  }

  #call<T>(f: (r: wasm.NativeReader) => T): T {
    return guarded(this.#r, f);
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
  get typed(): boolean {
    return this.#call((r) => r.typed);
  }
  get closed(): boolean {
    return this.#r.dead || this.#call((r) => r.closed);
  }
  coverData(): TokenBatch {
    return this.#call((r) => r.coverData());
  }
  nextBatch(n: number): Batch | null {
    return this.#call((r) => r.nextBatch(n));
  }
  /** Idempotent, and a no-op on a reader that trapped (its memory is lost until the page reloads). */
  close(): void {
    if (!this.#r.dead) this.#call((r) => r.close());
  }
}
