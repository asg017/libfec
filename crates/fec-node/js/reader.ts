import * as native from "../native/native.js";
import { callNative, FecError, MissingMappingError } from "./errors.js";
import type { Cover, Itemization } from "./generated/index.js";
import { toHeader, type Header } from "./header.js";
import {
  Converters,
  InvalidValueSink,
  RowBuilder,
  type DatesOption,
  type InvalidValue,
  type OnUnmapped,
  type Row,
} from "./row.js";
import { normalizeSource, type Source } from "./source.js";
import { TokenDecoder } from "./tokens.js";

/** Options for {@link open} and {@link read}. */
export interface OpenOptions {
  /**
   * How date columns of {@link Row.values} come out: `"iso"` (default) for
   * `"YYYY-MM-DD"` strings, `"date"` for `Date`s at UTC midnight. Typed
   * records ({@link Row.itemization}, {@link FilingReader.cover}) always use
   * ISO strings.
   */
  dates?: DatesOption;
  /**
   * What `rows()` does with a row whose type has no column mapping in the
   * filing's version: `"throw"` (default) a {@link MissingMappingError}, or
   * `"skip"` it and list it in {@link FilingReader.skippedRows}.
   * `itemizations()` only yields rows it can type, so this doesn't apply.
   */
  unknownRows?: "throw" | "skip";
  /** Rows per native batch (default 1024). A tuning knob; results don't depend on it. */
  batchSize?: number;
}

/** A row as raw strings, from {@link FilingReader.records}. */
export interface RawRecord {
  /** The row type as filed (field 0, trimmed). */
  rowType: string;
  /** The row's 1-based physical line. */
  line: number;
  /** Every field, as filed; `fields[0]` is the row type. */
  fields: string[];
}

/**
 * The six cover fields every filing has, whatever its form: the same as
 * Python's `Cover`. Dates follow the `dates` option; an invalid date is `null`.
 */
export interface CoverSummary {
  /** The cover's form type as filed: `"F3XN"`, `"F99"`. */
  readonly formType: string;
  /** The filer's FEC ID. */
  readonly filerId: string;
  /** The filer's name (for an individual filing Form 5 or 9, their name). */
  readonly filerName: string;
  /** The report code (`"Q1"`, `"M8"`), for forms that have one. */
  readonly reportCode: string | null;
  readonly coverageFromDate: string | Date | null;
  readonly coverageThroughDate: string | Date | null;
}

/** A row type with no mapping, skipped by `rows()` with `unknownRows: "skip"`. */
export interface SkippedRow {
  rowType: string;
  line: number;
}

const DEFAULT_BATCH_SIZE = 1024;

const FILTER_REMOVED =
  "filters were removed; switch on row.itemization.type or test row.rowType";

type Consumer = "itemizations" | "rows" | "records";

/** Validate options; `TypeError` on anything unknown. @internal */
export function checkOptions(options: OpenOptions): Required<OpenOptions> {
  const { dates = "iso", unknownRows = "throw", batchSize = DEFAULT_BATCH_SIZE } = options;
  if (dates !== "iso" && dates !== "date") {
    throw new TypeError(`dates must be "iso" or "date", got ${JSON.stringify(dates)}`);
  }
  if (unknownRows !== "throw" && unknownRows !== "skip") {
    throw new TypeError(`unknownRows must be "throw" or "skip", got ${JSON.stringify(unknownRows)}`);
  }
  if (!Number.isInteger(batchSize) || batchSize < 1 || batchSize > 2 ** 31) {
    throw new TypeError(`batchSize must be a positive integer, got ${batchSize}`);
  }
  return { dates, unknownRows, batchSize };
}

/** @internal */
export function openNative(source: Source): native.NativeReader {
  const src = normalizeSource(source);
  return src.kind === "path"
    ? callNative(() => native.NativeReader.openPath(src.path), { path: src.path })
    : callNative(() => native.NativeReader.openBytes(src.bytes));
}

/**
 * Open a filing for streaming. The header and cover are read now; rows are
 * read as you iterate, one batch at a time.
 *
 * Read it with **one** of {@link FilingReader.itemizations},
 * {@link FilingReader.rows} or {@link FilingReader.records}, then close it:
 *
 * ```ts
 * const filing = open("1805248.fec");
 * try {
 *   for (const row of filing.itemizations()) {
 *     switch (row.itemization.type) {
 *       case "ScheduleA": console.log(row.itemization.contribution_amount); break;
 *     }
 *   }
 * } finally {
 *   filing.close();
 * }
 * ```
 *
 * On Node 24+, Deno and Bun, `using filing = open(…)` closes it for you.
 *
 * A bytes source is read in place: don't modify it while the reader is open.
 *
 * @param source - A path, a `file:` URL, or the file's bytes.
 * @throws A Node-style error (`code: "ENOENT"`, …) if the path can't be opened,
 *   or {@link FecParseError} if the header or cover can't be parsed.
 */
export function open(source: Source, options: OpenOptions = {}): FilingReader {
  const opts = checkOptions(options);
  return new FilingReader(openNative(source), opts);
}

/**
 * A filing being streamed; see {@link open}. Its `header`, `cover`,
 * `coverSummary` and `coverRow` are read up front and stay available after
 * {@link close}.
 */
export class FilingReader implements Iterable<Row<null>>, Disposable {
  /** The filing ID from the file name (`"1805248"` for `FEC-1805248.fec`); `null` for bytes. */
  readonly id: string | null;
  /** The filing's format version: `"8.4"`, `"3.00"`, `"P3.4"` (paper). */
  readonly fecVersion: string;
  /** The filing's header. */
  readonly header: Header;
  /**
   * The typed cover record, narrowed with `switch (filing.cover?.type)`;
   * `null` when the form has no typed struct.
   */
  readonly cover: Cover | null;
  /** The six cover fields every form has. */
  readonly coverSummary: CoverSummary;
  /** The cover record as a raw {@link Row}. */
  readonly coverRow: Row<null>;

  readonly #native: native.NativeReader;
  readonly #options: Required<OpenOptions>;
  readonly #sink = new InvalidValueSink();
  readonly #skipped: SkippedRow[] = [];
  readonly #builder: RowBuilder;
  #consumer: Consumer | null = null;
  #closed = false;

  /** @internal Use {@link open}. */
  constructor(reader: native.NativeReader, options: Required<OpenOptions>) {
    this.#native = reader;
    this.#options = options;
    this.id = reader.id;
    this.fecVersion = reader.fecVersion;
    this.header = toHeader(reader.header);
    this.#builder = new RowBuilder(this.fecVersion, new Converters(this.#sink, options.dates));
    const [cover] = new TokenDecoder().decode(callNative(() => reader.coverData()));
    this.cover = cover as Cover | null;
    const s = reader.coverSummary;
    const date = (iso: string | null) =>
      iso !== null && options.dates === "date" ? new Date(`${iso}T00:00:00Z`) : iso;
    this.coverSummary = Object.freeze({
      formType: s.formType,
      filerId: s.filerId,
      filerName: s.filerName,
      reportCode: s.reportCode,
      coverageFromDate: date(s.coverageFromDate),
      coverageThroughDate: date(s.coverageThroughDate),
    });
    this.coverRow = this.#builder.row(reader.coverFields, reader.coverLine, null, () => {
      throw new FecError(`no column mapping for the cover ${s.formType}`, { code: "FEC_PARSE" });
    })!;
  }

  /** Amounts and dates that didn't parse, in file order: the first 1,000. */
  get invalidValues(): readonly InvalidValue[] {
    return this.#sink.list;
  }

  /** How many amounts and dates didn't parse, uncapped. */
  get invalidValueCount(): number {
    return this.#sink.count;
  }

  /** Rows `rows()` skipped with `unknownRows: "skip"`. */
  get skippedRows(): readonly SkippedRow[] {
    return this.#skipped;
  }

  /** Whether {@link close} has been called. */
  get closed(): boolean {
    return this.#closed;
  }

  /**
   * Every row that has a typed record ({@link Itemization}), as a
   * `Row<Itemization>`: narrow it with `switch (row.itemization.type)`.
   * Rows without one (summary rows, schedules with no struct yet) are
   * skipped. Typing every row costs about three times {@link rows}.
   *
   * @throws {@link FecError} `ERR_FILING_CONSUMED` if the reader was already
   *   read, `ERR_FILING_CLOSED` if it is closed.
   */
  itemizations(): Generator<Row<Itemization>, void, undefined> {
    if (arguments.length > 0) throw new TypeError(FILTER_REMOVED);
    this.#claim("itemizations");
    callNative(() => this.#native.setTyped(true));
    return this.#iterate(new TokenDecoder()) as Generator<Row<Itemization>, void, undefined>;
  }

  /**
   * Every row after the cover, untyped (`row.itemization` is `null`): the
   * fast path when you only need {@link Row.values}.
   *
   * @throws {@link MissingMappingError} for a row type with no mapping (see
   *   {@link OpenOptions.unknownRows}); {@link FecError} `ERR_FILING_CONSUMED`
   *   or `ERR_FILING_CLOSED` as for {@link itemizations}.
   */
  rows(): Generator<Row<null>, void, undefined> {
    if (arguments.length > 0) throw new TypeError(FILTER_REMOVED);
    this.#claim("rows");
    return this.#iterate(null) as Generator<Row<null>, void, undefined>;
  }

  /** Every row after the cover as raw strings, with no column mapping needed. */
  records(): Generator<RawRecord, void, undefined> {
    if (arguments.length > 0) throw new TypeError(FILTER_REMOVED);
    this.#claim("records");
    return this.#records();
  }

  /** `for (const row of filing)` is {@link rows}. */
  [Symbol.iterator](): Generator<Row<null>, void, undefined> {
    return this.rows();
  }

  /** Release the file (or buffer). Idempotent; the header and cover stay readable. */
  close(): void {
    this.#closed = true;
    this.#native.close();
  }

  /** = {@link close}, for `using`. */
  [Symbol.dispose](): void {
    this.close();
  }

  #claim(consumer: Consumer): void {
    if (this.#closed) throw new FecError("the filing is closed", { code: "ERR_FILING_CLOSED" });
    if (this.#consumer !== null) {
      throw new FecError(
        `this reader was already read with ${this.#consumer}(); open the file again to read it again`,
        { code: "ERR_FILING_CONSUMED" },
      );
    }
    this.#consumer = consumer;
  }

  #nextBatch(): native.Batch | null {
    if (this.#closed) throw new FecError("the filing is closed", { code: "ERR_FILING_CLOSED" });
    return callNative(() => this.#native.nextBatch(this.#options.batchSize));
  }

  *#iterate(decoder: TokenDecoder | null): Generator<Row<Itemization | null>, void, undefined> {
    const onUnmapped: OnUnmapped =
      this.#options.unknownRows === "skip"
        ? (rowType, line) => (this.#skipped.push({ rowType, line }), "skip")
        : (rowType, line) => {
            throw new MissingMappingError(rowType, this.fecVersion, line);
          };
    for (let batch; (batch = this.#nextBatch()); ) {
      for (const row of this.#builder.build(batch, decoder, onUnmapped)) {
        if (this.#closed) throw new FecError("the filing is closed", { code: "ERR_FILING_CLOSED" });
        yield row;
      }
    }
  }

  *#records(): Generator<RawRecord, void, undefined> {
    for (let batch; (batch = this.#nextBatch()); ) {
      const { text, ends, rowEnds, lines } = batch;
      let f = 0;
      let pos = 0;
      for (let i = 0; i < rowEnds.length; i++) {
        const fields: string[] = [];
        for (; f < rowEnds[i]!; f++) {
          fields.push(text.slice(pos, ends[f]!));
          pos = ends[f]!;
        }
        if (this.#closed) throw new FecError("the filing is closed", { code: "ERR_FILING_CLOSED" });
        yield { rowType: (fields[0] ?? "").trim(), line: lines[i]!, fields };
      }
    }
  }
}
