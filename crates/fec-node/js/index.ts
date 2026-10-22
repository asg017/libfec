/**
 * Parse FEC electronic filings (`.fec`) in Node, Deno and Bun.
 *
 * ```ts
 * import { readHeader } from "@asg017/libfec";
 *
 * const header = readHeader("1805248.fec");
 * console.log(header.fecVersion, header.softwareName);
 * ```
 *
 * @module
 */
import * as native from "./native.js";

export { readHeader, type Header } from "./header.js";
export { FecError, FecParseError, MissingMappingError } from "./errors.js";
export type { Source } from "./source.js";

/** The package version (lockstep with the libfec workspace). */
export const version: string = native.version();

// The typed covers and itemizations (types, ITEMIZATION_TYPES, COVER_TYPES,
// label functions), generated from fec-parser by `make gen-types`.
export * from "./generated/index.js";
