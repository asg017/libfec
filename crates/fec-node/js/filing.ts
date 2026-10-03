import * as native from "#native";
import { callNative, MissingMappingError } from "./errors.js";
import type { Cover, Itemization } from "./generated/index.js";
import type { Header } from "./header.js";
import {
  checkOptions,
  openReader,
  type CoverSummary,
  type FilingReader,
  type OpenOptions,
  type ResolvedOptions,
  type SkippedRow,
} from "./reader.js";
import type { InvalidValue, Row } from "./row.js";
import type { Source } from "./source.js";

/**
 * A whole filing read into memory by {@link read}. Re-iterable
 * (`for (const row of filing)` walks {@link rows}), and there is nothing to close.
 */
export class Filing implements Iterable<Row<null>> {
  /** The filing ID; see {@link FilingReader.id}. */
  readonly id: string | null;
  /** The filing's format version. */
  readonly fecVersion: string;
  /** The filing's header. */
  readonly header: Header;
  /** The typed cover; see {@link FilingReader.cover}. */
  readonly cover: Cover | null;
  /** The six cover fields every form has. */
  readonly coverSummary: CoverSummary;
  /** The cover record as a raw {@link Row}. */
  readonly coverRow: Row<null>;
  /** Every row after the cover, untyped (`itemization` is `null`). */
  readonly rows: readonly Row<null>[];
  /** Amounts and dates in {@link rows} that didn't parse: the first 1,000. */
  readonly invalidValues: readonly InvalidValue[];
  /** How many amounts and dates didn't parse, uncapped. */
  readonly invalidValueCount: number;
  /** Rows skipped with `unknownRows: "skip"`. */
  readonly skippedRows: readonly SkippedRow[];

  readonly #source: Source;
  readonly #options: ResolvedOptions;
  #itemizations: Row<Itemization>[] | undefined;

  /** @internal Use {@link read}. */
  constructor(reader: FilingReader, source: Source, options: ResolvedOptions) {
    this.id = reader.id;
    this.fecVersion = reader.fecVersion;
    this.header = reader.header;
    this.cover = reader.cover;
    this.coverSummary = reader.coverSummary;
    this.coverRow = reader.coverRow;
    this.rows = Object.freeze([...reader.rows()]);
    this.invalidValues = reader.invalidValues;
    this.invalidValueCount = reader.invalidValueCount;
    this.skippedRows = reader.skippedRows;
    this.#source = source;
    this.#options = options;
  }

  /** `rows.length`. */
  get length(): number {
    return this.rows.length;
  }

  /**
   * The typed rows (see {@link FilingReader.itemizations}). The first call
   * reads the source again, typing every row, and caches the result: later
   * calls return the same array. A bytes source is kept alive for this; a
   * path that changed or vanished since {@link read} gives different rows or
   * `ENOENT`.
   */
  itemizations(): readonly Row<Itemization>[] {
    if (this.#itemizations === undefined) {
      const reader = openReader(this.#source, this.#options);
      try {
        this.#itemizations = Object.freeze([...reader.itemizations()]) as Row<Itemization>[];
      } finally {
        reader.close();
      }
    }
    return this.#itemizations;
  }

  /** Iterate {@link rows}; a `Filing` can be iterated any number of times. */
  [Symbol.iterator](): IterableIterator<Row<null>> {
    return this.rows.values();
  }
}

/**
 * Read a whole filing: header, cover and every row (untyped, the fast path).
 * {@link Filing.itemizations} gives the typed rows on demand. For big
 * filings, {@link open} streams instead.
 *
 * @throws as {@link open}, plus whatever {@link FilingReader.rows} throws.
 */
export function read(source: Source, options: OpenOptions = {}): Filing {
  const opts = checkOptions(options);
  const reader = openReader(source, opts);
  try {
    return new Filing(reader, source, opts);
  } finally {
    reader.close();
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
  /** How {@link Row.values} reads it. */
  kind: "text" | "amount" | "date";
}

const columnCache = new Map<string, readonly Column[]>();

/**
 * The columns of a row type in an FEC version, in order: the keys of
 * {@link Row.values} for such rows.
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
