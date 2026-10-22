/**
 * The FEC e-filing RSS feed: new filings, about a week's worth, newest
 * first. Parsing is the `fec-rss` Rust crate (the same parser `libfec rss`
 * uses); {@link fetch} is a thin wrapper over the runtime's own `fetch()`.
 *
 * ```ts
 * import { rss } from "@asg017/libfec";
 *
 * const feed = await rss.fetch({ forms: ["F3X"], states: "CA" });
 * for (const item of feed.items) {
 *   console.log(item.filingId, item.committeeName, item.pubDate);
 * }
 *
 * rss.parseFeed(xml); // bring your own HTTP client
 * ```
 *
 * @module
 */
import * as native from "./native.js";
import { callNative } from "./errors.js";

/** One new filing in the feed. */
export interface Item {
  /** `"New filing by <committee name>"`. */
  readonly title: string;
  /** The filing's `.fec` URL on docquery.fec.gov. */
  readonly link: string;
  /** The item's HTML description, including the `*********CommitteeId: …` trailer. */
  readonly description: string;
  /** `pubDate`; `null` if missing or unparseable. */
  readonly pubDate: Date | null;
  /** The item's `guid`. */
  readonly guid: string;
  /** The filer's committee ID (`"C00536425"`). */
  readonly committeeId: string | null;
  /** The filing ID (`"2019253"`): `open()` the `.fec` at {@link Item.link}. */
  readonly filingId: string | null;
  /** The form type as filed (`"F3XN"`). */
  readonly formType: string | null;
  /** Start of the coverage period, `"YYYY-MM-DD"`. */
  readonly coverageFrom: string | null;
  /** End of the coverage period, `"YYYY-MM-DD"`. */
  readonly coverageThrough: string | null;
  /** The report type (`"OCTOBER QUARTERLY"`). */
  readonly reportType: string | null;
  /** {@link Item.title} without its `"New filing by "` prefix. */
  readonly committeeName: string;
}

/** A parsed feed: channel metadata plus its items, newest first. */
export interface Feed {
  /** The channel title (`"FEC Electronic Filing RSS Feed - ALL"`). */
  readonly title: string;
  /** The channel link. */
  readonly link: string;
  /** The channel description. */
  readonly description: string;
  /** The items, newest first. */
  readonly items: readonly Item[];
}

/** The feed's predefined filters. */
export type Preset = "all" | "monthly" | "quarterly" | "presidential" | "congressional" | "pac";

const PRESETS: readonly string[] = ["all", "monthly", "quarterly", "presidential", "congressional", "pac"];

/**
 * Which feed to read: a {@link Preset}, or custom filters. Any of
 * `committees`, `forms`, `states` or `parties` switches to the feed's
 * custom-filter format, which ignores `preset`.
 */
export interface FeedOptions {
  /** A predefined filter (case-insensitive). Default `"all"`. */
  preset?: Preset;
  /** Committee IDs (`"C00505412"`). */
  committees?: string | Iterable<string>;
  /** Form types (`"F3X"`). */
  forms?: string | Iterable<string>;
  /** Two-letter state codes. */
  states?: string | Iterable<string>;
  /** Party codes (`"DEM"`). */
  parties?: string | Iterable<string>;
}

/** {@link fetch}'s options: the feed filters, plus how to fetch. */
export interface FetchOptions extends FeedOptions {
  /** Abort the request. Default: a 60-second timeout. */
  signal?: AbortSignal;
}

function join(value: string | Iterable<string> | undefined): string | null {
  if (value === undefined || typeof value === "string") return value ?? null;
  return [...value].join(",");
}

/**
 * The feed URL for a preset or custom filters.
 *
 * @throws RangeError for an unknown `preset`.
 */
export function feedUrl(options: FeedOptions = {}): string {
  const preset = (options.preset ?? "all").toLowerCase();
  if (!PRESETS.includes(preset)) {
    throw new RangeError(`unknown preset ${JSON.stringify(options.preset)}: expected ${PRESETS.join(", ")}`);
  }
  return callNative(() =>
    native.rssFeedUrl(
      preset,
      join(options.committees),
      join(options.forms),
      join(options.states),
      join(options.parties),
    ),
  );
}

/**
 * Parse the feed's XML: a string, or its UTF-8 bytes.
 *
 * @throws {@link FecParseError} if it isn't a well-formed feed.
 */
export function parseFeed(xml: string | Uint8Array | ArrayBuffer): Feed {
  let feed: native.NativeRssFeed;
  if (typeof xml === "string") feed = callNative(() => native.parseRssText(xml));
  else if (xml instanceof Uint8Array) feed = callNative(() => native.parseRssBytes(xml));
  else if (xml instanceof ArrayBuffer) feed = callNative(() => native.parseRssBytes(new Uint8Array(xml)));
  else throw new TypeError("parseFeed() takes a string, a Uint8Array or an ArrayBuffer");
  return Object.freeze({
    title: feed.title,
    link: feed.link,
    description: feed.description,
    items: Object.freeze(
      feed.items.map((i) => Object.freeze({ ...i, pubDate: i.pubDate === null ? null : new Date(i.pubDate) })),
    ),
  });
}

/**
 * Download and parse the feed, with the runtime's global `fetch()`. See
 * {@link FeedOptions} for the filters.
 *
 * @throws The `fetch()` error on a network failure or abort; an `Error` for
 * a non-2xx response; {@link FecParseError} if the body isn't a feed.
 */
export async function fetch(options: FetchOptions = {}): Promise<Feed> {
  const url = feedUrl(options);
  const response = await globalThis.fetch(url, { signal: options.signal ?? AbortSignal.timeout(60_000) });
  if (!response.ok) {
    throw new Error(`GET ${url}: ${response.status} ${response.statusText}`);
  }
  return parseFeed(new Uint8Array(await response.arrayBuffer()));
}
