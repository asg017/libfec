// Records: the public `Record` class, and the internal builder that turns
// native batches (src/reader.rs) into records.
//
// `record.values` is built by one compiled factory per (row type, FEC
// version): an object literal with the columns in order, so every record of a
// type shares one object shape (plans/nodejs/01-bindings.md: ~2x faster than
// a loop). Where code generation is blocked the factory is a loop with the
// same output.
//
// The class is named `Record`, like Python's, so inside this module (and any
// module that imports it) TypeScript's global `Record<K, V>` utility type is
// shadowed: write `{ [k: string]: V }` instead.

import * as native from "./native.js";
import { callNative, MissingMappingError } from "./errors.js";
import type { Itemization } from "./generated/index.js";
import { TokenDecoder } from "./tokens.js";

/**
 * A value in {@link Record.values}:
 *
 * | Column kind | Value |
 * |---|---|
 * | text | the raw string, untrimmed (`""` stays `""`) |
 * | amount | a `number`; `null` if blank; the raw string if it doesn't parse |
 * | date | `"YYYY-MM-DD"` (or a `Date` with `dates: "date"`); `null` if blank or invalid |
 * | any, past the end of a short record | `null` |
 *
 * An unparseable amount keeps its raw string, so it stays detectable
 * (`typeof v === "string"` in an amount column).
 */
export type Value = string | number | Date | null;

/** A record's values by FEC column name (snake_case), in column order. */
export type RecordValues = { [column: string]: Value };

/** How {@link Record.toJSON} serializes a record. */
export interface RecordJSON {
  /** {@link Record.rowType} */
  rowType: string;
  /** {@link Record.line} */
  line: number;
  /** {@link Record.values} */
  values: RecordValues;
}

/**
 * One raw record of a filing: the cover line ({@link Filing.coverRecord}) or
 * a record after it ({@link Filing.itemizationRecords}): an itemization
 * (`SA11AI`, `SB17`, …), a text record or a summary record. Mirrors Python's
 * `libfec.Record`.
 *
 * All data is in own enumerable properties, so `{ ...record }` and
 * `structuredClone(record)` give plain objects (without the methods and the
 * lazy {@link itemization}).
 */
export class Record {
  /**
   * The row type as filed, field 0: `"SA11AI"`, `"SB17"`, `"TEXT"`. A `TEXT`
   * record's first column is named `rec_type`, not `form_type`, so test
   * `rowType` rather than `values.form_type`.
   */
  readonly rowType: string;
  /** The record's 1-based physical line in the file. */
  readonly line: number;
  /** The record's values by FEC column name, in column order; see {@link Value}. */
  readonly values: RecordValues;
  /** Every raw field of the record, in file order. */
  readonly fields: readonly string[];

  readonly #fecVersion: string;
  readonly #nameDelimiter: string | null;

  /** @internal Built by {@link Filing}; not meant to be constructed directly. */
  constructor(
    rowType: string,
    line: number,
    values: RecordValues,
    fields: readonly string[],
    fecVersion: string,
    nameDelimiter: string | null,
  ) {
    this.rowType = rowType;
    this.line = line;
    this.values = values;
    this.fields = fields;
    this.#fecVersion = fecVersion;
    this.#nameDelimiter = nameDelimiter;
  }

  /**
   * The record as a typed itemization (`ScheduleA`, …), or `null` for a
   * record that is not one or whose schedule has no type yet. Typed anew on
   * every access (a new object each time); {@link Filing.itemizations} is
   * the fast path for typing every record.
   *
   * Values are the parser's: an unparseable amount reads `0` (or `null`
   * where the field is optional) and an invalid date `null`, the same as
   * Python's typed classes. {@link values} keeps the raw text of such fields.
   */
  get itemization(): Itemization | null {
    const batch = callNative(() =>
      native.typeRecord(this.fields as string[], this.#fecVersion, this.#nameDelimiter),
    );
    // One value: the loop factories, not codegen, which only pays off over many.
    return new TokenDecoder({ codegen: false }).decode(batch)[0] as Itemization | null;
  }

  /**
   * Fields past the last mapped column; `[]` unless one of them is
   * non-empty (trailing empty fields are common and meaningless).
   */
  get extraFields(): string[] {
    const extra = this.fields.slice(Object.keys(this.values).length);
    return extra.some((f) => f !== "") ? extra : [];
  }

  /** The value of `column`, or `undefined` if the record has no such column. */
  get(column: string): Value | undefined {
    return Object.hasOwn(this.values, column) ? this.values[column] : undefined;
  }

  /** `{ rowType, line, values }` (`fields` is left out). */
  toJSON(): RecordJSON {
    return { rowType: this.rowType, line: this.line, values: this.values };
  }
}

// ---- value rules ----------------------------------------------------------------

// Rust's `f64::from_str` grammar (what the Python bindings use), not JS
// `Number()`, which also takes "0x10", "0b1", "" and "Infinity".
const AMOUNT = /^[+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?$/;
const SPECIAL = /^([+-]?)(inf|infinity|nan)$/i;

/** Rust-grammar amount, or `undefined` if `t` (trimmed, non-empty) isn't one. */
export function parseAmount(t: string): number | undefined {
  if (AMOUNT.test(t)) return Number(t);
  const m = SPECIAL.exec(t);
  if (m === null) return undefined;
  if (m[2]!.toLowerCase() === "nan") return NaN;
  return m[1] === "-" ? -Infinity : Infinity;
}

const DAYS = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/** The digit at `i` of `t`, or -1. */
function digit(t: string, i: number): number {
  const c = t.charCodeAt(i) - 48;
  return c >= 0 && c <= 9 ? c : -1;
}

/**
 * `YYYYMMDD` (trimmed) as `y * 10000 + m * 100 + d` if it is a real
 * calendar date in years 1..=9999, else -1. Digit arithmetic, no regex: this runs for every
 * date field.
 */
function dateNumber(t: string): number {
  if (t.length !== 8) return -1;
  let n = 0;
  for (let i = 0; i < 8; i++) {
    const d = digit(t, i);
    if (d < 0) return -1;
    n = n * 10 + d;
  }
  const y = Math.floor(n / 10000);
  const mo = Math.floor(n / 100) % 100;
  const d = n % 100;
  // Years below 1 are invalid in every binding: Python's `date` can't hold them.
  if (y < 1 || mo < 1 || mo > 12 || d < 1) return -1;
  const leap = mo === 2 && y % 4 === 0 && (y % 100 !== 0 || y % 400 === 0);
  return d > DAYS[mo - 1]! + (leap ? 1 : 0) ? -1 : n;
}

/** `YYYYMMDD` (trimmed) → `[y, m, d]` if it is a real calendar date. */
export function parseDate(t: string): [number, number, number] | undefined {
  const n = dateNumber(t);
  if (n < 0) return undefined;
  return [Math.floor(n / 10000), Math.floor(n / 100) % 100, n % 100];
}

/** How date columns of {@link Record.values} come out: ISO strings or `Date`s. */
export type DatesOption = "iso" | "date";

/** The converters a values factory calls; one per filing. */
export class Converters {
  constructor(readonly dates: DatesOption) {}

  amount(raw: string | undefined): Value {
    if (raw === undefined || raw === "") return null;
    const t = raw.trim();
    if (t === "") return null;
    const n = parseAmount(t);
    return n !== undefined ? n : raw;
  }

  date(raw: string | undefined): Value {
    if (raw === undefined || raw === "") return null;
    // Trim only when needed: almost every date is exactly 8 digits.
    const t = raw.length === 8 ? raw : raw.trim();
    if (t === "") return null;
    const n = dateNumber(t);
    if (n < 0) return null;
    if (this.dates === "date") {
      const date = new Date(0);
      date.setUTCFullYear(Math.floor(n / 10000), (Math.floor(n / 100) % 100) - 1, n % 100);
      return date;
    }
    return `${t.slice(0, 4)}-${t.slice(4, 6)}-${t.slice(6)}`;
  }
}

// ---- factories --------------------------------------------------------------------

type ValuesFactory = (a: readonly string[]) => RecordValues;

interface Schema {
  names: string[];
  build: ValuesFactory;
}

let codegenWorks: boolean | undefined;

/** Whether `new Function` is allowed here (checked once). @internal */
export function canCodegen(): boolean {
  if (codegenWorks === undefined) {
    try {
      codegenWorks = new Function("return 1")() === 1;
    } catch {
      codegenWorks = false;
    }
  }
  return codegenWorks;
}

function compileValues(columns: native.Column[], c: Converters, codegen: boolean): ValuesFactory {
  if (!codegen) {
    return (a) => {
      const o: RecordValues = {};
      for (let i = 0; i < columns.length; i++) {
        const { name, kind } = columns[i]!;
        o[name] =
          kind === "amount" ? c.amount(a[i]) : kind === "date" ? c.date(a[i]) : (a[i] ?? null);
      }
      return o;
    };
  }
  const props = columns.map(({ name, kind }, i) => {
    const key = JSON.stringify(name);
    switch (kind) {
      case "amount":
        return `${key}: c.amount(a[${i}])`;
      case "date":
        return `${key}: c.date(a[${i}])`;
      default:
        return `${key}: a[${i}] ?? null`;
    }
  });
  return new Function("c", `return (a) => ({${props.join(", ")}});`)(c) as ValuesFactory;
}

/**
 * Builds records for one filing, from raw fields or native batches: owns the
 * schema cache and the converters. Shared by every pass. @internal
 */
export class RecordBuilder {
  readonly #schemas = new Map<string, Schema | null>();
  readonly #converters: Converters;
  readonly #codegen: boolean;

  constructor(
    readonly fecVersion: string,
    readonly nameDelimiter: string | null,
    dates: DatesOption,
    options: { codegen?: boolean } = {},
  ) {
    this.#converters = new Converters(dates);
    this.#codegen = options.codegen ?? canCodegen();
  }

  /** The values factory for a row type, or `null` if it has no mapping. */
  schema(rowType: string): Schema | null {
    let s = this.#schemas.get(rowType);
    if (s === undefined) {
      let columns: native.Column[] | null;
      try {
        columns = callNative(() => native.schema(rowType, this.fecVersion));
      } catch (e) {
        if ((e as { code?: string }).code !== "FEC_MISSING_MAPPING") throw e;
        columns = null;
      }
      s =
        columns === null
          ? null
          : {
              names: columns.map((col) => col.name),
              build: compileValues(columns, this.#converters, this.#codegen),
            };
      this.#schemas.set(rowType, s);
    }
    return s;
  }

  /** One record from its raw fields, or the error for a row type with no mapping. */
  record(fields: string[], line: number): Record | MissingMappingError {
    // Trimmed: paper P2.3–P3.1 pad row types (`SB23 `), as the parser does.
    const rowType = (fields[0] ?? "").trim();
    const schema = this.schema(rowType);
    if (schema === null) return new MissingMappingError(rowType, this.fecVersion, line);
    return new Record(rowType, line, schema.build(fields), fields, this.fecVersion, this.nameDelimiter);
  }

  /**
   * Every record of a batch, in order. A row with no mapping comes out as
   * its {@link MissingMappingError}, for the pass to throw when it gets
   * there (so the records before it are still yielded).
   */
  build(batch: native.Batch): (Record | MissingMappingError)[] {
    const { text, ends, rowEnds, lines } = batch;
    const out = new Array<Record | MissingMappingError>(rowEnds.length);
    let f = 0;
    let pos = 0;
    for (let i = 0; i < rowEnds.length; i++) {
      const end = rowEnds[i]!;
      const fields = new Array<string>(end - f);
      for (let k = 0; f < end; f++, k++) {
        const e = ends[f]!;
        fields[k] = text.slice(pos, e);
        pos = e;
      }
      out[i] = this.record(fields, lines[i]!);
    }
    return out;
  }
}

/** Row `i`'s raw fields in a batch (random access, for the rare row that needs them). @internal */
export function batchFields(batch: native.Batch, i: number): string[] {
  const { text, ends, rowEnds } = batch;
  const start = i === 0 ? 0 : rowEnds[i - 1]!;
  const end = rowEnds[i]!;
  const fields = new Array<string>(end - start);
  let pos = start === 0 ? 0 : ends[start - 1]!;
  for (let f = start, k = 0; f < end; f++, k++) {
    const e = ends[f]!;
    fields[k] = text.slice(pos, e);
    pos = e;
  }
  return fields;
}
