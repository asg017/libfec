import * as native from "../native/native.js";

/** The package version (lockstep with the libfec workspace). */
export const version: string = native.version();

export { FecError, FecParseError, MissingMappingError } from "./errors.js";
export { readHeader, type Header } from "./header.js";
export type { Source } from "./source.js";

// The typed covers and itemizations (types, ITEMIZATION_TYPES, COVER_TYPES,
// label functions), generated from fec-parser by `make gen-types`.
export * from "./generated/index.js";
