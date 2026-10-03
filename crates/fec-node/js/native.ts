// The one module in js/ that touches the platform: the napi-rs binding and
// the `node:*` APIs. Everything else imports it as `#native` (package.json
// "imports"), so a future browser build can map `#native` to a wasm adapter
// with the same exports (plans/nodejs/06-wasm-spike.md, W1).

import { constants } from "node:os";
import { fileURLToPath } from "node:url";

export * from "../native/native.js";

/** A `file:` URL as a filesystem path. */
export function urlToPath(url: URL): string {
  return fileURLToPath(url);
}

/** Node's (negative) `errno` for a system error code like `"ENOENT"`, if known. */
export function errnoOf(code: string): number | undefined {
  const n = (constants.errno as Record<string, number | undefined>)[code];
  return n === undefined ? undefined : -n;
}
