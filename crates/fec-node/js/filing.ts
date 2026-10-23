import * as native from "./native.js";
import { callNative, FecError, MissingMappingError } from "./errors.js";
import { fetchExplained, inBrowser, proxied, withTimeout, type ProxyOption } from "./fetch.js";
import type { Cover, Itemization } from "./generated/index.js";
import { toHeader, type Header } from "./header.js";
import { batchFields, Record, RecordBuilder, type DatesOption } from "./record.js";
import { normalizeSource, type Source } from "./source.js";
import { TokenDecoder } from "./tokens.js";
import { Unknown } from "./unknown.js";

/** Options for {@link Filing}. */
export interface FilingOptions {
  /**
   * How date columns of {@link Record.values} and the coverage dates
   * ({@link Filing.coverageFromDate}, …) come out: `"iso"` (default) for
   * `"YYYY-MM-DD"` strings, `"date"` for `Date`s at UTC midnight. Typed
   * records ({@link Filing.itemizations}, {@link Filing.cover}) always use
   * ISO strings.
   */
  dates?: DatesOption;
  /**
   * The filing's ID, for {@link Filing.id}. Bytes have no file name to take
   * it from, so without this their `id` is `null`; for a path it replaces
   * the ID taken from the file name.
   *
   * ```js
   * new Filing(bytes, { id: "1805248" }).id // "1805248"
   * ```
   */
  id?: string;
  /** Rows per native batch (default 1024). A tuning knob; results don't depend on it. */
  batchSize?: number;
}

/** Options for {@link Filing.fromId}: the {@link FilingOptions} (but `id`), plus how to fetch. */
export interface FromIdOptions extends Omit<FilingOptions, "id"> {
  /**
   * Fetch through a proxy. docquery.fec.gov sends no CORS headers, so a
   * browser page can't read a filing directly. Not needed in Node, Deno or
   * Bun.
   *
   * - a string containing `{url}`: that placeholder is replaced by the
   *   URL-encoded filing URL (`"https://proxy.example/fetch?u={url}"`);
   * - any other string: a prefix for the URL-encoded filing URL
   *   (`"https://proxy.example/?url="`);
   * - a function: given the filing URL, returns the URL to fetch.
   */
  proxy?: ProxyOption;
  /** Abort the download. Combined with {@link timeout}: whichever comes first. */
  signal?: AbortSignal;
  /** Milliseconds before the download is aborted (default 60 000; `Infinity` for none). */
  timeout?: number;
}

/** `1721696`, `"1721696"` or `"FEC-1721696"` → `"1721696"`, like Python's `normalize_filing_id`. */
function normalizeFilingId(filingId: unknown): string {
  // A number goes through `String()`, so a negative one (or `1.5`) fails the
  // digit check below like any other malformed id.
  if (typeof filingId !== "number" && typeof filingId !== "string") {
    throw new TypeError(`filing id must be a number or a string, got ${filingId === null ? "null" : typeof filingId}`);
  }
  const raw = String(filingId);
  const trimmed = raw.trim();
  const id = trimmed.startsWith("FEC-") ? trimmed.slice(4) : trimmed.startsWith("FEC") ? trimmed.slice(3) : trimmed;
  if (!/^[0-9]+$/.test(id)) throw new TypeError(`not an FEC filing id: ${JSON.stringify(raw)}`);
  return id;
}

const DEFAULT_TIMEOUT = 60_000;

interface ResolvedOptions {
  dates: DatesOption;
  id: string | null;
  batchSize: number;
}

const DEFAULT_BATCH_SIZE = 1024;
const OPTION_KEYS = new Set(["dates", "id", "batchSize"]);

/** Validate options; `TypeError` on anything unknown. */
function checkOptions(options: FilingOptions): ResolvedOptions {
  if (options === null || typeof options !== "object") {
    throw new TypeError(`options must be an object, got ${options === null ? "null" : typeof options}`);
  }
  for (const key of Object.keys(options)) {
    if (!OPTION_KEYS.has(key)) {
      throw new TypeError(`unknown option ${JSON.stringify(key)} (expected dates, id or batchSize)`);
    }
  }
  const { dates = "iso", batchSize = DEFAULT_BATCH_SIZE, id } = options;
  if (id !== undefined && typeof id !== "string") {
    throw new TypeError(`id must be a string, got ${typeof id}`);
  }
  if (dates !== "iso" && dates !== "date") {
    throw new TypeError(`dates must be "iso" or "date", got ${JSON.stringify(dates)}`);
  }
  if (!Number.isInteger(batchSize) || batchSize < 1 || batchSize > 2 ** 31) {
    throw new TypeError(`batchSize must be a positive integer, got ${batchSize}`);
  }
  return { dates, batchSize, id: id ?? null };
}

/**
 * One running pass's native reader, as {@link Filing.close} sees it. It holds
 * the reader, never the generator, so an abandoned pass can be collected.
 */
interface LivePass {
  readonly reader: native.NativeReader;
  /** The filing's set of live passes, to leave when the pass is collected. */
  readonly live: Set<LivePass>;
}

/**
 * Closes the reader of a pass whose generator was dropped mid-pass (started
 * with `next()`, never finished or `return()`ed). The target is the token in
 * the generator's frame, so the reader (and its file descriptor or wasm
 * memory) goes when the generator does, not at {@link Filing.close}. The held
 * {@link LivePass} doesn't reference the token or the generator.
 */
const abandoned = new FinalizationRegistry<LivePass>((pass) => {
  pass.live.delete(pass);
  pass.reader.close();
});

function closedError(): FecError {
  return new FecError("the filing is closed", { code: "ERR_FILING_CLOSED" });
}

/**
 * One FEC filing, opened for streaming. Mirrors Python's `libfec.Filing`.
 *
 * The header and the cover are read when it is constructed, so
 * {@link header}, {@link cover}, {@link coverRecord} and the filer attributes
 * ({@link formType}, {@link filerId}, …) are there up front. The records after
 * the cover are streamed, a batch at a time, by {@link itemizations} (typed)
 * or {@link itemizationRecords} (raw), one-to-one and in file order. Each
 * call is a new pass from the top, and passes are independent: they can run
 * one after the other or interleaved.
 *
 * ```ts
 * using filing = new Filing("1805248.fec");
 * console.log(filing.filerName, filing.coverageThroughDate);
 * for (const item of filing.itemizations()) {
 *   switch (item.type) {
 *     case "ScheduleA": console.log(item.contribution_amount); break;
 *     case "Unknown": break; // a record with no typed class, e.g. F3PS
 *   }
 * }
 * ```
 *
 * `using` (Node 24+, Deno, Bun) closes it; elsewhere call {@link close}.
 * A bytes source is read in place: don't modify it while the filing is open.
 *
 * @throws A Node-style error (`code: "ENOENT"`, …) if the path can't be
 *   opened, or {@link FecParseError} if the header or cover can't be parsed.
 */
export class Filing implements Disposable {
  /**
   * The filing ID: {@link FilingOptions.id} if given, else from the file name
   * (`"1805248"` for `FEC-1805248.fec`); `null` for bytes without an `id`.
   */
  readonly id: string | null;
  /** The filing's format version: `"8.4"`, `"3.00"`, `"P3.4"` (paper). */
  readonly fecVersion: string;
  /** The filing's header. */
  readonly header: Header;
  /** The source's size in bytes (`0` if unknown). */
  readonly sourceLength: number;
  /** The cover's form type as filed: `"F3XN"`, `"F99"`. */
  readonly formType: string;
  /** The filer's FEC ID, whatever the form calls it. */
  readonly filerId: string;
  /** The filer's name, whatever the form calls it. */
  readonly filerName: string;
  /** `"Q1"`, `"M8"`, …; `null` for a form that is not a periodic report. */
  readonly reportCode: string | null;
  /** First day the report covers; follows the `dates` option. `null` if absent or invalid. */
  readonly coverageFromDate: string | Date | null;
  /** Last day the report covers; follows the `dates` option. `null` if absent or invalid. */
  readonly coverageThroughDate: string | Date | null;
  /**
   * The typed cover (`Form3X`, `Form99`, …), narrowed with
   * `switch (filing.cover.type)`, or an {@link Unknown} for a form with no
   * typed struct. The same object every time.
   */
  readonly cover: Cover | Unknown;
  /** The cover line as a raw {@link Record}. The same object every time. */
  readonly coverRecord: Record;

  readonly #source: native.NativeSource;
  readonly #options: ResolvedOptions;
  readonly #builder: RecordBuilder;
  /** The path given, for the errors of later passes; `undefined` for bytes. */
  readonly #path: string | undefined;
  /**
   * The readers of passes running now, for {@link close} to stop. Strong refs
   * to readers only; a pass whose generator is collected leaves the set (see
   * {@link abandoned}).
   */
  readonly #live = new Set<LivePass>();
  #closed = false;

  /**
   * @param source - A path, a `file:` URL, or the file's bytes.
   */
  constructor(source: Source, options: FilingOptions = {}) {
    const opts = checkOptions(options);
    const src = normalizeSource(source);
    this.#path = src.kind === "path" ? src.path : undefined;
    this.#source =
      src.kind === "path"
        ? callNative(() => native.NativeSource.openPath(src.path), { path: src.path })
        : callNative(() => native.NativeSource.openBytes(src.bytes));
    try {
      const reader = callNative(() => this.#source.reader(false));
      try {
        this.#options = opts;
        this.id = opts.id ?? this.#source.id;
        this.sourceLength = this.#source.sourceLength;
        this.fecVersion = reader.fecVersion;
        const header = reader.header;
        this.header = toHeader(header);
        this.#builder = new RecordBuilder(this.fecVersion, header.nameDelimiter, opts.dates);
        const s = reader.coverSummary;
        const date = (iso: string | null) =>
          iso !== null && opts.dates === "date" ? new Date(`${iso}T00:00:00Z`) : iso;
        this.formType = s.formType;
        this.filerId = s.filerId;
        this.filerName = s.filerName;
        this.reportCode = s.reportCode;
        this.coverageFromDate = date(s.coverageFromDate);
        this.coverageThroughDate = date(s.coverageThroughDate);
        const cover = this.#builder.record(reader.coverFields, reader.coverLine);
        if (cover instanceof MissingMappingError) {
          throw new FecError(`no column mapping for the cover ${s.formType}`, { code: "FEC_PARSE" });
        }
        this.coverRecord = cover;
        const [typed] = new TokenDecoder({ codegen: false }).decode(callNative(() => reader.coverData()));
        this.cover = (typed as Cover | null) ?? new Unknown(cover);
      } finally {
        reader.close();
      }
    } catch (e) {
      this.#source.close();
      throw e;
    }
  }

  /**
   * Download an electronic filing from docquery.fec.gov
   * (`https://docquery.fec.gov/dcdev/posted/<id>.fec`) into memory and open
   * it, with {@link Filing.id} set. Mirrors Python's `Filing.from_id`; like
   * it, there is no fallback to paper filings.
   *
   * ```ts
   * const filing = await Filing.fromId(1721696); // or "1721696", "FEC-1721696"
   * console.log(filing.filerName);
   * ```
   *
   * In a browser, pass a {@link FromIdOptions.proxy}: docquery sends no CORS
   * headers. Without one, the request fails with a `TypeError` that says so
   * (`code: "ERR_CORS"`).
   *
   * @param id - The filing ID: digits, optionally prefixed `FEC-` or `FEC`.
   * @throws `TypeError` for a malformed id or option, before any request;
   *   {@link FecError} `ERR_FILING_NOT_FOUND` (`status: 404`) if there is no
   *   such filing, `ERR_HTTP` (with `status`) for any other non-2xx response;
   *   the `fetch()` error on a network failure, an abort or the timeout
   *   (`TimeoutError`); otherwise as the {@link Filing} constructor.
   */
  static async fromId(id: number | string, options: FromIdOptions = {}): Promise<Filing> {
    const filingId = normalizeFilingId(id);
    if (options === null || typeof options !== "object") {
      throw new TypeError(`options must be an object, got ${options === null ? "null" : typeof options}`);
    }
    const { proxy, signal, timeout = DEFAULT_TIMEOUT, ...filingOptions } = options;
    if ("id" in filingOptions) {
      throw new TypeError("fromId() takes the filing id as its first argument, not an id option");
    }
    checkOptions(filingOptions);
    if (typeof timeout !== "number" || Number.isNaN(timeout) || timeout <= 0) {
      throw new TypeError(`timeout must be a positive number of milliseconds, got ${String(timeout)}`);
    }
    if (signal !== undefined && !(signal instanceof AbortSignal)) {
      throw new TypeError("signal must be an AbortSignal");
    }
    const target = `https://docquery.fec.gov/dcdev/posted/${filingId}.fec`;
    const url = proxied(target, proxy);
    // A browser ignores User-Agent at best, and at worst preflights for it.
    const headers: { [name: string]: string } = inBrowser() ? {} : { "User-Agent": `libfec-js/${native.version()}` };
    const deadline = withTimeout(signal, timeout);
    let bytes: Uint8Array;
    try {
      const response = await fetchExplained(url, { headers, signal: deadline.signal }, proxy);
      if (!response.ok) {
        const status = response.status;
        void response.body?.cancel().catch(() => {});
        if (status === 404) {
          throw new FecError(`no filing ${filingId} on docquery.fec.gov (GET ${url}: 404)`, {
            code: "ERR_FILING_NOT_FOUND",
            status,
          });
        }
        throw new FecError(`GET ${url}: ${status} ${response.statusText}`, { code: "ERR_HTTP", status });
      }
      bytes = new Uint8Array(await response.arrayBuffer());
    } finally {
      deadline.cleanup();
    }
    return new Filing(bytes, { ...filingOptions, id: filingId });
  }

  /** Whether {@link close} has been called. */
  get closed(): boolean {
    return this.#closed;
  }

  /**
   * Every record after the cover, typed, in file order: an {@link Itemization}
   * (`ScheduleA`, …), or an {@link Unknown} for a record with no typed class
   * (a summary record, a schedule not typed yet). Narrow with
   * `switch (item.type)`. One-to-one with {@link itemizationRecords}.
   *
   * Each call is a new, independent pass from the top of the filing.
   *
   * @throws {@link FecError} `ERR_FILING_CLOSED` if the filing is closed
   *   (now, or on the next step of a running pass);
   *   {@link MissingMappingError} for a record with no typed class whose row
   *   type has no column mapping; {@link FecParseError} for a row that can't
   *   be parsed.
   */
  itemizations(): Generator<Itemization | Unknown, void, undefined> {
    if (arguments.length > 0) {
      throw new TypeError("itemizations() takes no arguments; switch on item.type");
    }
    if (this.#closed) throw closedError();
    return this.#itemizations();
  }

  /**
   * Every record after the cover as a raw {@link Record}, in file order:
   * the same records as {@link itemizations}, one-to-one. The fast path when
   * you only need {@link Record.values}; {@link Record.itemization} types one
   * on demand.
   *
   * Each call is a new, independent pass from the top of the filing.
   *
   * @throws {@link MissingMappingError} for a row type with no column
   *   mapping; otherwise as {@link itemizations}.
   */
  itemizationRecords(): Generator<Record, void, undefined> {
    if (arguments.length > 0) {
      throw new TypeError("itemizationRecords() takes no arguments; test record.rowType");
    }
    if (this.#closed) throw closedError();
    return this.#records();
  }

  /**
   * Release the source and stop every pass: a running {@link itemizations}
   * or {@link itemizationRecords} throws `ERR_FILING_CLOSED` on its next
   * step. Idempotent; the header, cover and filer attributes stay readable.
   */
  close(): void {
    this.#closed = true;
    for (const pass of this.#live) pass.reader.close();
    this.#live.clear();
    this.#source.close();
  }

  /** = {@link close}, for `using`. */
  [Symbol.dispose](): void {
    this.close();
  }

  /**
   * A new native pass, opened on the generator's first step and tracked so
   * {@link close} can stop it. The generator keeps the returned token in its
   * frame until {@link release}: the token's reachability is the generator's.
   */
  #open(typed: boolean): { pass: LivePass } {
    if (this.#closed) throw closedError();
    const reader = callNative(() => this.#source.reader(typed), { path: this.#path });
    const pass: LivePass = { reader, live: this.#live };
    const token = { pass };
    this.#live.add(pass);
    abandoned.register(token, pass, token);
    return token;
  }

  /** End a pass: exhausted, `return()`ed or thrown. */
  #release(token: { pass: LivePass }): void {
    abandoned.unregister(token);
    this.#live.delete(token.pass);
    token.pass.reader.close();
  }

  #nextBatch(reader: native.NativeReader): native.Batch | null {
    if (this.#closed) throw closedError();
    return callNative(() => reader.nextBatch(this.#options.batchSize));
  }

  *#records(): Generator<Record, void, undefined> {
    const token = this.#open(false);
    const reader = token.pass.reader;
    try {
      for (let batch; (batch = this.#nextBatch(reader)); ) {
        for (const r of this.#builder.build(batch)) {
          if (this.#closed) throw closedError();
          if (r instanceof MissingMappingError) throw r;
          yield r;
        }
      }
    } finally {
      this.#release(token);
    }
  }

  *#itemizations(): Generator<Itemization | Unknown, void, undefined> {
    const token = this.#open(true);
    const reader = token.pass.reader;
    try {
      const decoder = new TokenDecoder();
      for (let batch; (batch = this.#nextBatch(reader)); ) {
        const items = decoder.decode(batch.typed!) as (Itemization | null)[];
        for (let i = 0; i < items.length; i++) {
          if (this.#closed) throw closedError();
          const item = items[i]!;
          if (item !== null) {
            yield item;
            continue;
          }
          // Only an untyped row needs its fields and values.
          const r = this.#builder.record(batchFields(batch, i), batch.lines[i]!);
          if (r instanceof MissingMappingError) throw r;
          yield new Unknown(r);
        }
      }
    } finally {
      this.#release(token);
    }
  }
}

/**
 * A column of a row type's layout; see {@link columns}.
 *
 * @category Columns
 */
export interface Column {
  /** The FEC column name: `"contribution_amount"`. */
  name: string;
  /** How {@link Record.values} reads it. */
  kind: "text" | "amount" | "date";
}

const columnCache = new Map<string, readonly Column[]>();

/**
 * The columns of a row type in an FEC version, in order: the keys of
 * {@link Record.values} for such records.
 *
 * ```ts
 * columns("SA11AI", "8.4")[20] // { name: "contribution_amount", kind: "amount" }
 * ```
 *
 * @throws {@link MissingMappingError} (with `line: 0`) if there is no mapping.
 *
 * @category Columns
 */
export function columns(rowType: string, fecVersion: string): Column[] {
  if (typeof rowType !== "string" || typeof fecVersion !== "string") {
    throw new TypeError(`columns() takes two strings, got ${typeof rowType} and ${typeof fecVersion}`);
  }
  const key = `${rowType}\0${fecVersion}`;
  let cols = columnCache.get(key);
  if (cols === undefined) {
    try {
      cols = callNative(() => native.schema(rowType, fecVersion)) as Column[];
    } catch (e) {
      if ((e as { code?: string }).code === "FEC_MISSING_MAPPING") {
        throw new MissingMappingError(rowType, fecVersion, 0);
      }
      throw e;
    }
    columnCache.set(key, cols);
  }
  return cols.map((c) => ({ ...c }));
}
