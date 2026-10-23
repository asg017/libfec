/**
 * Parse FEC electronic filings (`.fec`) in Node, Deno and Bun.
 *
 * ```ts
 * import { Filing } from "@alex.garcia/libfec";
 *
 * using filing = new Filing("1805248.fec");
 * console.log(filing.formType, filing.filerName);
 * for (const item of filing.itemizations()) {
 *   switch (item.type) {
 *     case "ScheduleA": console.log(item.contribution_amount); break;
 *     case "Unknown": break; // a record with no typed class
 *   }
 * }
 * ```
 *
 * Each call to {@link Filing.itemizations} or {@link Filing.itemizationRecords}
 * is a new pass; {@link Filing.fromId} downloads a filing by its ID (pass a
 * `proxy` in a browser). Importing {@link Record} shadows TypeScript's global
 * `Record<K, V>` in that module: import it under another name
 * (`import { Record as FecRecord }`) if you need both.
 *
 * @module
 */
import * as native from "./native.js";

export { Filing, columns, type FilingOptions, type FromIdOptions, type Column } from "./filing.js";
export type { ProxyOption } from "./fetch.js";
export { Record, type Value, type RecordValues, type RecordJSON, type DatesOption } from "./record.js";
export { Unknown, type UnknownJSON } from "./unknown.js";
export { readHeader, type Header } from "./header.js";
export { FecError, FecParseError, MissingMappingError } from "./errors.js";
export type { Source } from "./source.js";
/** The FEC e-filing RSS feed: `rss.fetch()`, `rss.parseFeed()`, `rss.feedUrl()`. */
export * as rss from "./rss.js";

/** The package version (lockstep with the libfec workspace). */
export const version: string = native.version();

// The typed covers and itemizations (types, ITEMIZATION_TYPES, COVER_TYPES,
// label functions), generated from fec-parser by `make gen-types`.
export * from "./generated/index.js";
