import * as native from "../native/native.js";
import { normalizeSource, type Source } from "./source.js";

/** A filing's header: the `HDR` record, or a 1.x/2.x `/* Header` block. */
export interface Header {
  /** `"HDR"`, or `"/*"` for a 1.x/2.x `/* Header` block. */
  readonly recordType: string;
  /** `"FEC"`; empty for paper filings. */
  readonly efType: string;
  /** The format version, e.g. `"8.4"` (`"P3.4"` for paper). */
  readonly fecVersion: string;
  readonly softwareName: string;
  readonly softwareVersion: string;
  readonly reportId: string | null;
  readonly reportNumber: string | null;
  readonly comment: string | null;
  /** How the header is written: `"hdr"`, `"legacy_block"` or `"paper"`. */
  readonly style: "hdr" | "legacy_block" | "paper";
  /** Body field delimiter: `"fs"` (0x1C) or `"comma"`. */
  readonly delimiter: "fs" | "comma";
  /** Sub-delimiter of combined name fields (3.x–5.x and 1.x/2.x only). */
  readonly nameDelimiter: string | null;
  /** Paper filings: FEC data-entry batch number. */
  readonly batchNumber: string | null;
  /** Paper filings P2.6+: date the FEC received the filing. */
  readonly receivedDate: string | null;
  /** Whether this is FEC data entry of a paper filing. */
  readonly isPaper: boolean;
  /** 1.x/2.x `/* Header` block: every `key = value` line before `Schedule_Counts:`, in file order. Empty otherwise. */
  readonly legacyFields: Readonly<Record<string, string>>;
  /** 1.x/2.x `/* Header` block: the `Schedule_Counts:` lines (row type → count as written). Empty otherwise. */
  readonly scheduleCounts: Readonly<Record<string, string>>;
}

/** Build the public, frozen `Header` from the native one. */
export function toHeader(h: native.NativeHeader): Header {
  return Object.freeze({
    ...h,
    style: h.style as Header["style"],
    delimiter: h.delimiter as Header["delimiter"],
    legacyFields: toRecord(h.legacyFields),
    scheduleCounts: toRecord(h.scheduleCounts),
  });
}

function toRecord(fields: native.NativeHeaderField[]): Readonly<Record<string, string>> {
  const out: Record<string, string> = {};
  for (const { key, value } of fields) out[key] = value;
  return Object.freeze(out);
}

/**
 * Read only a filing's header. Nothing after it is read, so this works on a
 * filing whose cover or rows can't be parsed.
 */
export function readHeader(source: Source): Header {
  const src = normalizeSource(source);
  return toHeader(
    src.kind === "path"
      ? native.readHeaderPath(src.path)
      : native.readHeaderBytes(src.bytes),
  );
}
