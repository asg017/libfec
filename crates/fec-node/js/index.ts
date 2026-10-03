import * as native from "../native/native.js";

/** The package version (lockstep with the libfec workspace). */
export const version: string = native.version();

export { readHeader, type Header } from "./header.js";
export type { Source } from "./source.js";
