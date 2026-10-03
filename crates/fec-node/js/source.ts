import { urlToPath } from "./native.js";

/**
 * Where a filing comes from. A `string` is always a path (never `.fec` text);
 * a `URL` must be a `file:` URL; bytes are the raw file contents.
 */
export type Source = string | URL | Uint8Array | ArrayBuffer;

/** A {@link Source}, normalized for the native layer. */
export type NormalizedSource =
  | { kind: "path"; path: string }
  | { kind: "bytes"; bytes: Uint8Array };

export function normalizeSource(source: Source): NormalizedSource {
  if (typeof source === "string") return { kind: "path", path: source };
  if (source instanceof URL) {
    if (source.protocol !== "file:") {
      throw new TypeError("only file: URLs are supported");
    }
    return { kind: "path", path: urlToPath(source) };
  }
  // Buffer is a Uint8Array subclass.
  if (source instanceof Uint8Array) return { kind: "bytes", bytes: source };
  if (source instanceof ArrayBuffer) {
    return { kind: "bytes", bytes: new Uint8Array(source) };
  }
  throw new TypeError(
    `source must be a path string, a file: URL, a Uint8Array or an ArrayBuffer, got ${describe(source)}`,
  );
}

function describe(value: unknown): string {
  if (value === null) return "null";
  if (typeof value === "object") return value.constructor?.name ?? "object";
  return typeof value;
}
