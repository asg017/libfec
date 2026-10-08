// The one module in js/ that touches the platform: the napi-rs binding and
// the `node:*` APIs. Everything else imports it as "./native.js", so a future
// browser build can swap this one file for a wasm adapter with the same
// exports (plans/nodejs/06-wasm-spike.md, W1). Not a package.json "imports"
// alias: Deno doesn't resolve those when the package is imported by path.

import { constants } from "node:os";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

export * from "../native/native.js";

/** A `file:` URL as a filesystem path. */
export function urlToPath(url: URL): string {
  return fileURLToPath(url);
}

/**
 * The path as this runtime's own `fs` errors print it in their message: as
 * given, except under Node on Windows, where libuv reports the resolved path.
 * Deno and Bun keep it as given everywhere.
 */
export function messagePath(path: string): string {
  const node = !("Deno" in globalThis) && !("Bun" in globalThis);
  return node && process.platform === "win32" ? resolve(path) : path;
}

/** Node's (negative) `errno` for a system error code like `"ENOENT"`, if known. */
export function errnoOf(code: string): number | undefined {
  const n = (constants.errno as Record<string, number | undefined>)[code];
  return n === undefined ? undefined : -n;
}
