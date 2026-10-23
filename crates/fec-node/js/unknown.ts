import type { Record, RecordValues } from "./record.js";

/** How {@link Unknown.toJSON} serializes it. */
export interface UnknownJSON {
  /** Always `"Unknown"`. */
  type: "Unknown";
  /** {@link Unknown.formType} */
  formType: string;
  /** The record's values by column name ({@link Record.values}). */
  values: RecordValues;
}

/**
 * A record with no typed class: a cover with no struct, a summary record
 * (`F3PS`, …), or an itemization schedule not typed yet. Mirrors Python's
 * `libfec.Unknown`.
 *
 * It stands in for the typed object in {@link Filing.cover} and
 * {@link Filing.itemizations}, so neither is ever `null`. `type` is
 * `"Unknown"`, so `switch (item.type)` narrows `Itemization | Unknown`:
 *
 * ```ts
 * for (const item of filing.itemizations()) {
 *   switch (item.type) {
 *     case "ScheduleA": console.log(item.contribution_amount); break;
 *     case "Unknown": console.log(item.formType, item.record.values); break;
 *   }
 * }
 * ```
 */
export class Unknown {
  /** The discriminant: always `"Unknown"`. */
  readonly type = "Unknown" as const;
  /** The row type as filed, e.g. `"F3PS"` ({@link Record.rowType}). */
  readonly formType: string;
  /** The underlying raw record. */
  readonly record: Record;

  /** @internal Built by {@link Filing}; not meant to be constructed directly. */
  constructor(record: Record) {
    this.formType = record.rowType;
    this.record = record;
  }

  /** `{ type, formType, values }`: the record's values by column name, like Python's `to_dict()`. */
  toJSON(): UnknownJSON {
    return { type: this.type, formType: this.formType, values: this.record.values };
  }
}
