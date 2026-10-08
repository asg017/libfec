import { errnoOf, messagePath } from "./native.js";

/**
 * Base class of every error this package raises about a filing. `code` says
 * which kind:
 *
 * | `code` | When |
 * |---|---|
 * | `FEC_PARSE` | {@link FecParseError}: the header, cover or a row can't be parsed |
 * | `FEC_MISSING_MAPPING` | {@link MissingMappingError}: no column mapping for a row type |
 * | `ERR_FILING_CLOSED` | reading a {@link FilingReader} after `close()` |
 * | `ERR_FILING_CONSUMED` | a second `itemizations()`, `rows()`, `records()` or `for…of` on one reader |
 *
 * A missing or unreadable path is not a `FecError`: it throws the same error
 * Node's `fs.openSync` would (`code: "ENOENT"`, `path`, `syscall: "open"`).
 *
 * @category Errors
 */
export class FecError extends Error {
  /** What went wrong; see the table above. */
  code?: string;

  constructor(message: string, options?: { code?: string; cause?: unknown }) {
    super(message, options?.cause === undefined ? undefined : { cause: options.cause });
    this.name = new.target.name;
    if (options?.code !== undefined) this.code = options.code;
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
 * columns can't be named. Raised by `rows()` (unless `unknownRows: "skip"`)
 * and {@link columns}. `code: "FEC_MISSING_MAPPING"`.
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

/** A Node-style system error, shaped like `fs.openSync`'s. */
function systemError(code: string, path: string, cause: unknown): Error {
  const err = new Error(`${code}: ${IO_CODES[code]}, open '${messagePath(path)}'`, { cause });
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
export function fromNative(e: unknown, context: { path?: string } = {}): unknown {
  // Only the `code` matters, not how the binding built the error, so any
  // backend that throws `{ code, message }` (napi, a wasm adapter) maps alike.
  if (e === null || typeof e !== "object" || e instanceof FecError) return e;
  const { code, message } = e as { code?: unknown; message?: unknown };
  const msg = typeof message === "string" ? message : String(code);
  switch (code) {
    case "FEC_PARSE":
      return new FecParseError(msg);
    case "ERR_FILING_CLOSED":
    case "ERR_FILING_CONSUMED":
    case "FEC_MISSING_MAPPING":
      return new FecError(msg, { code });
    default:
      if (typeof code === "string" && code in IO_CODES) {
        return systemError(code, context.path ?? "", e);
      }
      return e;
  }
}

/** Run a native call, converting what it throws. @internal */
export function callNative<T>(f: () => T, context?: { path?: string }): T {
  try {
    return f();
  } catch (e) {
    throw fromNative(e, context);
  }
}
