/**
 * Parse FEC electronic filings (`.fec`) in Node, Deno and Bun.
 *
 * ```ts
 * import { open } from "@asg017/libfec";
 *
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
 * @module
 */
import * as native from "../native/native.js";

export {
  open,
  FilingReader,
  type OpenOptions,
  type CoverSummary,
  type RawRecord,
  type SkippedRow,
} from "./reader.js";
export {
  Row,
  type Value,
  type RowValues,
  type RowJSON,
  type InvalidValue,
  type DatesOption,
} from "./row.js";
export { read, Filing, columns, type Column } from "./filing.js";
export { readHeader, type Header } from "./header.js";
export { FecError, FecParseError, MissingMappingError } from "./errors.js";
export type { Source } from "./source.js";

/** The package version (lockstep with the libfec workspace). */
export const version: string = native.version();

// The typed covers and itemizations (types, ITEMIZATION_TYPES, COVER_TYPES,
// label functions), generated from fec-parser by `make gen-types`.
export * from "./generated/index.js";
