// Rows: the public `Row` class, and the internal builder that turns native
// batches (src/reader.rs) into rows.
//
// `row.values` is built by one compiled factory per (row type, FEC version):
// an object literal with the columns in order, so every row of a type shares
// one object shape (plans/nodejs/01-bindings.md: ~2x faster than a loop).
// Where code generation is blocked the factory is a loop with the same output.

import * as native from "./native.js";
import { callNative } from "./errors.js";
import type { Itemization } from "./generated/index.js";
import type { TokenDecoder } from "./tokens.js";

/**
 * A value in {@link Row.values}:
 *
 * | Column kind | Value |
 * |---|---|
 * | text | the raw string, untrimmed (`""` stays `""`) |
 * | amount | a `number`; `null` if blank; the raw string if it doesn't parse |
 * | date | `"YYYY-MM-DD"` (or a `Date` with `dates: "date"`); `null` if blank or invalid |
 * | any, past the end of a short row | `null` |
 *
 * Unparseable amounts and invalid dates are also listed in the reader's
 * `invalidValues`.
 */
export type Value = string | number | Date | null;

/** A row's values by FEC column name (snake_case), in column order. */
export type RowValues = Record<string, Value>;

/** An amount or date that didn't parse; see `invalidValues` on a reader. */
export interface InvalidValue {
  /** The row's 1-based line. */
  line: number;
  /** The column name. */
  column: string;
  /** The field exactly as filed. */
  raw: string;
}

/** How {@link Row.toJSON} serializes a row. */
export interface RowJSON<I extends Itemization | null = Itemization | null> {
  /** {@link Row.rowType} */
  rowType: string;
  /** {@link Row.line} */
  line: number;
  /** {@link Row.values} */
  values: RowValues;
  /** {@link Row.itemization} */
  itemization: I;
}

/**
 * One row of a filing after the cover: an itemization (`SA11AI`, `SB17`, …),
 * a text record, or a summary row, mirroring Python's `libfec_parser.Row`.
 *
 * - From `itemizations()`, a row is a `Row<Itemization>`: {@link itemization}
 *   is the typed record, narrowed with `switch (row.itemization.type)`.
 * - From `rows()` it is a `Row<null>`: only the column-keyed {@link values}.
 *
 * All data is in own enumerable properties, so `{ ...row }` and
 * `structuredClone(row)` give plain objects (without the methods).
 */
export class Row<I extends Itemization | null = Itemization | null> {
  /**
   * The row type as filed, field 0: `"SA11AI"`, `"SB17"`, `"TEXT"`. A `TEXT`
   * row's first column is named `rec_type`, not `form_type`, so test
   * `rowType` rather than `values.form_type`.
   */
  readonly rowType: string;
  /** The row's 1-based physical line in the file. */
  readonly line: number;
  /** The row's values by FEC column name, in column order; see {@link Value}. */
  readonly values: RowValues;
  /**
   * The row as a typed record, from `itemizations()`; `null` from `rows()`.
   *
   * Values are the parser's: an unparseable amount reads `0` (or `null`
   * where the field is optional) and an invalid date `null`, the same as
   * Python's typed classes. {@link values} keeps the raw text of such
   * fields and the reader lists them in `invalidValues`.
   */
  readonly itemization: I;
  /** Every raw field of the row, in file order. */
  readonly fields: readonly string[];

  /** @internal Built by the readers; not meant to be constructed directly. */
  constructor(
    rowType: string,
    line: number,
    values: RowValues,
    itemization: I,
    fields: readonly string[],
  ) {
    this.rowType = rowType;
    this.line = line;
    this.values = values;
    this.itemization = itemization;
    this.fields = fields;
  }

  /**
   * Fields past the last mapped column; `[]` unless one of them is
   * non-empty (trailing empty fields are common and meaningless).
   */
  get extraFields(): string[] {
    const extra = this.fields.slice(Object.keys(this.values).length);
    return extra.some((f) => f !== "") ? extra : [];
  }

  /** The value of `column`, or `undefined` if the row has no such column. */
  get(column: string): Value | undefined {
    return Object.hasOwn(this.values, column) ? this.values[column] : undefined;
  }

  /** `{ rowType, line, values, itemization }` (`fields` is left out). */
  toJSON(): RowJSON<I> {
    return {
      rowType: this.rowType,
      line: this.line,
      values: this.values,
      itemization: this.itemization,
    };
  }
}

// ---- value rules ----------------------------------------------------------------

// Rust's `f64::from_str` grammar (what the Python bindings use), not JS
// `Number()`, which also takes "0x10", "0b1", "" and "Infinity".
const AMOUNT = /^[+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?$/;
const SPECIAL = /^([+-]?)(inf|infinity|nan)$/i;

/** Collects invalid amounts and dates: the first 1,000, plus a total count. */
export class InvalidValueSink {
  static readonly CAP = 1000;
  readonly list: InvalidValue[] = [];
  count = 0;

  push(line: number, column: string, raw: string): void {
    this.count++;
    if (this.list.length < InvalidValueSink.CAP) this.list.push({ line, column, raw });
  }
}

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
 * calendar date, else -1. Digit arithmetic, no regex: this runs for every
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
  if (mo < 1 || mo > 12 || d < 1) return -1;
  const leap = mo === 2 && y % 4 === 0 && (y % 100 !== 0 || y % 400 === 0);
  return d > DAYS[mo - 1]! + (leap ? 1 : 0) ? -1 : n;
}

/** `YYYYMMDD` (trimmed) → `[y, m, d]` if it is a real calendar date. */
export function parseDate(t: string): [number, number, number] | undefined {
  const n = dateNumber(t);
  if (n < 0) return undefined;
  return [Math.floor(n / 10000), Math.floor(n / 100) % 100, n % 100];
}

/** How date columns of {@link Row.values} come out: ISO strings or `Date`s. */
export type DatesOption = "iso" | "date";

/** The converters a values factory calls; one per reader (it owns the sink). */
export class Converters {
  constructor(
    readonly sink: InvalidValueSink,
    readonly dates: DatesOption,
  ) {}

  amount(raw: string | undefined, column: string, line: number): Value {
    if (raw === undefined || raw === "") return null;
    const t = raw.trim();
    if (t === "") return null;
    const n = parseAmount(t);
    if (n !== undefined) return n;
    this.sink.push(line, column, raw);
    return raw;
  }

  date(raw: string | undefined, column: string, line: number): Value {
    if (raw === undefined || raw === "") return null;
    // Trim only when needed: almost every date is exactly 8 digits.
    const t = raw.length === 8 ? raw : raw.trim();
    if (t === "") return null;
    const n = dateNumber(t);
    if (n < 0) {
      this.sink.push(line, column, raw);
      return null;
    }
    if (this.dates === "date") {
      const date = new Date(0);
      date.setUTCFullYear(Math.floor(n / 10000), (Math.floor(n / 100) % 100) - 1, n % 100);
      return date;
    }
    return `${t.slice(0, 4)}-${t.slice(4, 6)}-${t.slice(6)}`;
  }
}

// ---- factories --------------------------------------------------------------------

type ValuesFactory = (a: readonly string[], line: number) => RowValues;

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
    return (a, line) => {
      const o: RowValues = {};
      for (let i = 0; i < columns.length; i++) {
        const { name, kind } = columns[i]!;
        o[name] =
          kind === "amount"
            ? c.amount(a[i], name, line)
            : kind === "date"
              ? c.date(a[i], name, line)
              : (a[i] ?? null);
      }
      return o;
    };
  }
  const props = columns.map(({ name, kind }, i) => {
    const key = JSON.stringify(name);
    switch (kind) {
      case "amount":
        return `${key}: c.amount(a[${i}], ${key}, line)`;
      case "date":
        return `${key}: c.date(a[${i}], ${key}, line)`;
      default:
        return `${key}: a[${i}] ?? null`;
    }
  });
  return new Function("c", `return (a, line) => ({${props.join(", ")}});`)(c) as ValuesFactory;
}

/** What to do with a row whose type has no column mapping. */
export type OnUnmapped = (rowType: string, line: number) => "skip";

/**
 * Builds rows from native batches for one reader: owns the schema cache,
 * the converters (and so the invalid-value sink) and, in typed mode, the
 * token decoder. @internal
 */
export class RowBuilder {
  readonly #schemas = new Map<string, Schema | null>();
  readonly #converters: Converters;
  readonly #codegen: boolean;

  constructor(
    readonly fecVersion: string,
    converters: Converters,
    options: { codegen?: boolean } = {},
  ) {
    this.#converters = converters;
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

  /** One row from its raw fields (the cover row, tests). */
  row<I extends Itemization | null>(
    fields: string[],
    line: number,
    itemization: I,
    onUnmapped: OnUnmapped,
  ): Row<I> | null {
    // Trimmed: paper P2.3–P3.1 pad row types (`SB23 `), as the parser does.
    const rowType = (fields[0] ?? "").trim();
    const schema = this.schema(rowType);
    if (schema === null) {
      onUnmapped(rowType, line);
      return null;
    }
    return new Row(rowType, line, schema.build(fields, line), itemization, fields);
  }

  /**
   * Every row of a batch. With a `decoder`, the batch must be typed and each
   * row gets its decoded itemization; without, `itemization` is `null`.
   */
  build(
    batch: native.Batch,
    decoder: TokenDecoder | null,
    onUnmapped: OnUnmapped,
  ): Row<Itemization | null>[] {
    const { text, ends, rowEnds, lines } = batch;
    const items = decoder === null ? null : decoder.decode(batch.typed!);
    const out: Row<Itemization | null>[] = [];
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
      const row = this.row(
        fields,
        lines[i]!,
        (items === null ? null : items[i]) as Itemization | null,
        onUnmapped,
      );
      if (row !== null) out.push(row);
    }
    return out;
  }
}
