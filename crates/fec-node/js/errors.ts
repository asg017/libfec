import { errnoOf, messagePath } from "./native.js";

/**
 * Base class of every error this package raises about a filing. `code` says
 * which kind:
 *
 * | `code` | When |
 * |---|---|
 * | `FEC_PARSE` | {@link FecParseError}: the header, cover or a row can't be parsed |
 * | `FEC_MISSING_MAPPING` | {@link MissingMappingError}: no column mapping for a row type |
 * | `ERR_FILING_CLOSED` | reading a {@link Filing} after `close()` |
 * | `ERR_FILING_NOT_FOUND` | {@link Filing.fromId}: docquery has no such filing (HTTP 404); `status` is set |
 * | `ERR_HTTP` | {@link Filing.fromId}: any other non-2xx response; `status` is set |
 *
 * A missing or unreadable path is not a `FecError`: it throws the same error
 * Node's `fs.openSync` would (`code: "ENOENT"`, `path`, `syscall: "open"`).
 * Nor is a network failure: {@link Filing.fromId} and `rss.fetch` let the
 * runtime's `fetch()` error through, except that a browser's CORS block
 * becomes a `TypeError` with `code: "ERR_CORS"` that explains it.
 *
 * @category Errors
 */
export class FecError extends Error {
  /** What went wrong; see the table above. */
  code?: string;
  /** The HTTP status, for `ERR_FILING_NOT_FOUND` and `ERR_HTTP`. */
  status?: number;

  constructor(message: string, options?: { code?: string; status?: number; cause?: unknown }) {
    super(message, options?.cause === undefined ? undefined : { cause: options.cause });
    this.name = new.target.name;
    if (options?.code !== undefined) this.code = options.code;
    if (options?.status !== undefined) this.status = options.status;
  }
}

/**
 * The header, cover or a row of a filing can't be parsed. `code: "FEC_PARSE"`.
 *
 * @category Errors
 */
export class FecParseError extends FecError {
  constructor(message: string, options?: { cause?: unknown }) {
    super(message, { ...options, code: "FEC_PARSE" });
  }
}

/**
 * A row's type has no column mapping in the filing's FEC version, so its
 * columns can't be named. Raised by `itemizationRecords()`, by
 * `itemizations()` for a record with no typed class, and by {@link columns}.
 * `code: "FEC_MISSING_MAPPING"`.
 *
 * @category Errors
 */
export class MissingMappingError extends FecError {
  /**
   * @param rowType - The row type as filed (`"SA11AI"`).
   * @param fecVersion - The filing's format version (`"8.4"`).
   * @param line - The row's 1-based line, or 0 when there is none ({@link columns}).
   */
  constructor(
    readonly rowType: string,
    readonly fecVersion: string,
    readonly line: number,
  ) {
    super(
      `no column mapping for row type ${rowType} in FEC version ${fecVersion}` +
        (line > 0 ? ` (line ${line})` : ""),
      { code: "FEC_MISSING_MAPPING" },
    );
  }
}

const IO_CODES: Record<string, string> = {
  ENOENT: "no such file or directory",
  EACCES: "permission denied",
  EISDIR: "illegal operation on a directory",
  EIO: "i/o error",
};

/**
 * What a native call that may open a path needs for its errors: the path as
 * given, and optionally how its message prints it, fixed at an earlier time
 * (see {@link messagePath}: Node on Windows resolves it against the cwd).
 *
 * @internal
 */
export interface PathContext {
  path?: string;
  messagePath?: string;
}

/** A Node-style system error, shaped like `fs.openSync`'s. */
function systemError(code: string, context: PathContext, cause: unknown): Error {
  const path = context.path ?? "";
  const shown = context.messagePath ?? messagePath(path);
  const err = new Error(`${code}: ${IO_CODES[code]}, open '${shown}'`, { cause });
  const errno = errnoOf(code);
  return Object.assign(err, {
    code,
    ...(errno === undefined ? {} : { errno }),
    syscall: "open",
    path,
  });
}

/**
 * Convert an error thrown by the native module into the public class. Every
 * native call in `js/` goes through here.
 *
 * @internal
 */
export function fromNative(e: unknown, context: PathContext = {}): unknown {
  // Only the `code` matters, not how the binding built the error, so any
  // backend that throws `{ code, message }` (napi, a wasm adapter) maps alike.
  if (e === null || typeof e !== "object" || e instanceof FecError) return e;
  const { code, message } = e as { code?: unknown; message?: unknown };
  const msg = typeof message === "string" ? message : String(code);
  switch (code) {
    case "FEC_PARSE":
      return new FecParseError(msg);
    case "ERR_FILING_CLOSED":
    case "FEC_MISSING_MAPPING":
      return new FecError(msg, { code });
    default:
      if (typeof code === "string" && code in IO_CODES) {
        return systemError(code, context, e);
      }
      return e;
  }
}

/** Run a native call, converting what it throws. @internal */
export function callNative<T>(f: () => T, context?: PathContext): T {
  try {
    return f();
  } catch (e) {
    throw fromNative(e, context);
  }
}
