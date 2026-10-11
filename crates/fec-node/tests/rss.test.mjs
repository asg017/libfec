import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { FecParseError, rss } from "../dist/index.js";

// A trimmed copy of the live ?preDefinedFilingType=ALL feed (2026-10-10),
// shared with the fec-rss crate's and the Python bindings' tests.
const FEED = readFileSync(new URL("../../fec-rss/tests/fixtures/all.xml", import.meta.url));
const BASE = "https://efilingapps.fec.gov/rss/generate";

test("bytes, ArrayBuffer and string parse alike", () => {
  const a = rss.parseFeed(FEED);
  const b = rss.parseFeed(FEED.toString("utf8"));
  const c = rss.parseFeed(FEED.buffer.slice(FEED.byteOffset, FEED.byteOffset + FEED.byteLength));
  assert.deepEqual(a, b);
  assert.deepEqual(a, c);
});

test("feed", () => {
  const feed = rss.parseFeed(FEED);
  assert.equal(feed.title, "FEC Electronic Filing RSS Feed - ALL");
  assert.equal(feed.items.length, 30);
  assert.ok(Object.isFrozen(feed) && Object.isFrozen(feed.items) && Object.isFrozen(feed.items[0]));
});

test("item", () => {
  const item = rss.parseFeed(FEED).items[0];
  assert.equal(item.committeeName, "ALLIED UNIVERSAL PAC");
  assert.equal(item.filingId, "2019253");
  assert.equal(item.committeeId, "C00536425");
  assert.equal(item.formType, "F3XN");
  assert.equal(item.reportType, "OCTOBER QUARTERLY");
  assert.equal(item.coverageFrom, "2026-07-01");
  assert.equal(item.coverageThrough, "2026-09-30");
  assert.ok(item.pubDate instanceof Date);
  assert.equal(item.pubDate.toISOString(), "2026-10-10T17:50:22.000Z");
  assert.equal(item.link, "http://docquery.fec.gov/dcdev/posted/2019253.fec");
  assert.ok(item.description.startsWith("<p>The ALLIED UNIVERSAL PAC"));
  assert.deepEqual(Object.keys(item).sort(), [
    "committeeId", "committeeName", "coverageFrom", "coverageThrough", "description",
    "filingId", "formType", "guid", "link", "pubDate", "reportType", "title",
  ]);
});

test("XML entities are decoded", () => {
  const names = rss.parseFeed(FEED).items.map((i) => i.committeeName);
  assert.ok(names.includes("Marshall & Blue Origin Investments"));
});

test("errors: FecParseError for a bad feed or bad UTF-8, TypeError for a bad argument", () => {
  assert.throws(() => rss.parseFeed("<rss><channel><title>x</ti"), FecParseError);
  assert.throws(() => rss.parseFeed(new Uint8Array([0xff, 0xfe])), FecParseError);
  assert.throws(() => rss.parseFeed(123), TypeError);
});

test("feedUrl: presets and custom filters", () => {
  assert.equal(rss.feedUrl(), `${BASE}?preDefinedFilingType=ALL`);
  assert.equal(rss.feedUrl({ preset: "PAC" }), `${BASE}?preDefinedFilingType=F3X`);
  assert.equal(
    rss.feedUrl({ preset: "monthly", committees: ["C00505412", "C00513531"], forms: "f3x" }),
    `${BASE}?cids=C00505412,C00513531&forms=F3X`,
  );
  assert.equal(rss.feedUrl({ states: new Set(["CA"]) }), rss.feedUrl({ states: "CA" }));
  assert.throws(() => rss.feedUrl({ preset: "weekly" }), RangeError);
});

/** Run `body` with `globalThis.fetch` replaced by `stub`. */
async function withFetch(stub, body) {
  const real = globalThis.fetch;
  globalThis.fetch = stub;
  try {
    await body();
  } finally {
    globalThis.fetch = real;
  }
}

test("fetch: GETs feedUrl() and parses the body", async () => {
  const calls = [];
  await withFetch(async (url, init) => {
    calls.push({ url, init });
    return new Response(FEED);
  }, async () => {
    const feed = await rss.fetch({ preset: "pac" });
    assert.equal(feed.items.length, 30);
  });
  assert.equal(calls.length, 1);
  assert.equal(calls[0].url, `${BASE}?preDefinedFilingType=F3X`);
  assert.ok(calls[0].init.signal instanceof AbortSignal);
});

test("fetch: a non-2xx response is an Error; the caller's signal is passed through", async () => {
  const signal = new AbortController().signal;
  let seen;
  await withFetch(async (_url, init) => {
    seen = init.signal;
    return new Response("nope", { status: 503, statusText: "Service Unavailable" });
  }, async () => {
    await assert.rejects(rss.fetch({ signal }), /503 Service Unavailable/);
  });
  assert.equal(seen, signal);
});

test("fetch: proxy as a prefix, a {url} template or a function", async () => {
  const urls = [];
  await withFetch(async (url) => {
    urls.push(url);
    return new Response(FEED);
  }, async () => {
    await rss.fetch({ preset: "pac", proxy: "https://p.example/?url=" });
    await rss.fetch({ preset: "pac", proxy: "https://p.example/get/{url}/raw" });
    await rss.fetch({ preset: "pac", proxy: (u) => `https://p.example/${u.length}` });
  });
  const feed = encodeURIComponent(`${BASE}?preDefinedFilingType=F3X`);
  assert.deepEqual(urls, [
    `https://p.example/?url=${feed}`,
    `https://p.example/get/${feed}/raw`,
    `https://p.example/${BASE.length + "?preDefinedFilingType=F3X".length}`,
  ]);
  await assert.rejects(rss.fetch({ proxy: 42 }), TypeError);
});

test("fetch: in a browser, a failed request without a proxy explains CORS", async () => {
  const failed = async () => {
    throw new TypeError("Failed to fetch");
  };
  // Outside a browser the fetch error passes through as is.
  await withFetch(failed, async () => {
    await assert.rejects(rss.fetch(), { message: "Failed to fetch" });
  });
  globalThis.document = {};
  try {
    await withFetch(failed, async () => {
      await assert.rejects(rss.fetch(), (e) => e instanceof TypeError && /CORS/.test(e.message) && e.cause?.message === "Failed to fetch");
      // With a proxy, the error is the proxy's to explain.
      await assert.rejects(rss.fetch({ proxy: "https://p.example/?u=" }), { message: "Failed to fetch" });
    });
  } finally {
    delete globalThis.document;
  }
});
