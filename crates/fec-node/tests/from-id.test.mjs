// Filing.fromId() against a stubbed globalThis.fetch; the live download is in
// from-id.network.test.mjs (`make test-network`).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { Filing, FecError, FecParseError, rss, version } from "../dist/index.js";
import { fixture, leftovers } from "./helpers.mjs";

const PAC = readFileSync(fixture("1721696.fec"));
const URL_1721696 = "https://docquery.fec.gov/dcdev/posted/1721696.fec";

/** Run `body` with `globalThis.fetch` replaced by `stub`; returns the calls. */
async function withFetch(stub, body) {
  const calls = [];
  const real = globalThis.fetch;
  globalThis.fetch = async (url, init) => {
    calls.push({ url, init });
    return stub(url, init);
  };
  try {
    await body();
  } finally {
    globalThis.fetch = real;
  }
  return calls;
}

const ok = () => new Response(PAC);

/** Run `body` as if in a browser page. */
async function inBrowser(body) {
  globalThis.document = {};
  try {
    await body();
  } finally {
    delete globalThis.document;
  }
}

test("fromId: id spellings, URL, User-Agent, a working Filing with id set", async () => {
  for (const id of [1721696, "1721696", "FEC-1721696", " FEC1721696 "]) {
    let filing;
    const calls = await withFetch(ok, async () => {
      filing = await Filing.fromId(id);
    });
    assert.equal(calls.length, 1);
    assert.equal(calls[0].url, URL_1721696);
    assert.equal(calls[0].init.headers["User-Agent"], `libfec-js/${version}`);
    assert.ok(calls[0].init.signal instanceof AbortSignal);
    assert.ok(filing instanceof Filing);
    assert.equal(filing.id, "1721696");
    assert.equal(filing.filerName, "PFIZER INC. PAC");
    assert.equal(filing.sourceLength, PAC.length);
    let n = 0;
    for (const _ of filing.itemizations()) n++;
    assert.equal(n, 1387);
    filing.close();
  }
});

test("fromId: Filing options pass through", async () => {
  await withFetch(ok, async () => {
    const filing = await Filing.fromId(1721696, { dates: "date", batchSize: 7 });
    assert.ok(filing.coverageThroughDate instanceof Date);
    filing.close();
  });
});

test("fromId: a malformed id or option is a TypeError, before any request", async () => {
  const calls = await withFetch(ok, async () => {
    for (const bad of ["", "FEC-", "12a4", "C00900860", -5, 1.5, "1 2"]) {
      await assert.rejects(Filing.fromId(bad), { name: "TypeError", message: /not an FEC filing id/ }, String(bad));
    }
    for (const bad of [null, undefined, {}, 1721696n]) {
      await assert.rejects(Filing.fromId(bad), { name: "TypeError", message: /must be a number or a string/ });
    }
    await assert.rejects(Filing.fromId(1, null), TypeError);
    await assert.rejects(Filing.fromId(1, { id: "2" }), TypeError);
    await assert.rejects(Filing.fromId(1, { dates: "nope" }), TypeError);
    await assert.rejects(Filing.fromId(1, { bogus: 1 }), TypeError);
    for (const timeout of [0, -1, NaN, "5"]) {
      await assert.rejects(Filing.fromId(1, { timeout }), TypeError);
    }
    await assert.rejects(Filing.fromId(1, { signal: {} }), TypeError);
    await assert.rejects(Filing.fromId(1, { proxy: 42 }), TypeError);
  });
  assert.equal(calls.length, 0);
});

test("fromId: 404 is ERR_FILING_NOT_FOUND, other non-2xx ERR_HTTP, with status", async () => {
  await withFetch(() => new Response("", { status: 404, statusText: "Not Found" }), async () => {
    await assert.rejects(Filing.fromId(99999999999), (e) => {
      assert.ok(e instanceof FecError);
      assert.equal(e.code, "ERR_FILING_NOT_FOUND");
      assert.equal(e.status, 404);
      assert.match(e.message, /99999999999/);
      return true;
    });
  });
  await withFetch(() => new Response("", { status: 503, statusText: "Service Unavailable" }), async () => {
    await assert.rejects(Filing.fromId(1), (e) => {
      assert.ok(e instanceof FecError);
      assert.equal(e.code, "ERR_HTTP");
      assert.equal(e.status, 503);
      assert.match(e.message, /503 Service Unavailable/);
      return true;
    });
  });
});

test("fromId: an empty download is a parse error; a network error passes through", async () => {
  await withFetch(() => new Response(new Uint8Array(0)), async () => {
    await assert.rejects(Filing.fromId(1), FecParseError);
  });
  const boom = new TypeError("fetch failed");
  await withFetch(() => Promise.reject(boom), async () => {
    await assert.rejects(Filing.fromId(1), (e) => e === boom);
  });
});

/** A fetch that honours `init.signal` and otherwise never settles. */
const hang = (_url, init) =>
  new Promise((_resolve, reject) => {
    const signal = init.signal;
    if (signal.aborted) reject(signal.reason);
    signal.addEventListener("abort", () => reject(signal.reason), { once: true });
  });

test("fromId: timeout and signal abort, whichever comes first", async () => {
  await withFetch(hang, async () => {
    await assert.rejects(Filing.fromId(1, { timeout: 20 }), { name: "TimeoutError" });

    const controller = new AbortController();
    const pending = Filing.fromId(1, { signal: controller.signal, timeout: 60_000 });
    controller.abort(new Error("stop"));
    await assert.rejects(pending, { message: "stop" });

    // A caller's signal doesn't cancel the timeout.
    await assert.rejects(Filing.fromId(1, { signal: new AbortController().signal, timeout: 20 }), { name: "TimeoutError" });
    // An already-aborted signal aborts at once.
    await assert.rejects(Filing.fromId(1, { signal: AbortSignal.abort(new Error("early")) }), { message: "early" });
  });
  // timeout: Infinity and no signal: no signal at all.
  const calls = await withFetch(ok, async () => {
    (await Filing.fromId(1721696, { timeout: Infinity })).close();
  });
  assert.equal(calls[0].init.signal, undefined);
});

test("fromId: no timer or abort listener outlives a request, on any path", async () => {
  // The review's reproducer: a runtime without AbortSignal.any, a long-lived
  // signal, three 404s. Each used to leave a listener on the signal.
  const any = AbortSignal.any;
  AbortSignal.any = undefined;
  try {
    const { signal } = new AbortController();
    const left = await leftovers(12_345, signal, async () => {
      await withFetch(() => new Response("not found", { status: 404 }), async () => {
        for (let i = 0; i < 3; i++) {
          await assert.rejects(Filing.fromId(1, { signal, timeout: 12_345 }), { code: "ERR_FILING_NOT_FOUND" });
        }
      });
      await withFetch(ok, async () => {
        (await Filing.fromId(1721696, { signal, timeout: 12_345 })).close();
      });
      await withFetch(() => new Response(new Uint8Array(0)), async () => {
        await assert.rejects(Filing.fromId(1, { signal, timeout: 12_345 }), FecParseError);
      });
      await withFetch(() => Promise.reject(new TypeError("fetch failed")), async () => {
        await assert.rejects(Filing.fromId(1, { signal, timeout: 12_345 }), TypeError);
      });
    });
    assert.deepEqual(left, { timers: 0, listeners: 0 });
  } finally {
    AbortSignal.any = any;
  }
  // The default timeout, no signal: the timer is cleared too (an uncleared
  // one kept Deno running for 60 s after a fast download).
  const left = await leftovers(60_000, undefined, () =>
    withFetch(ok, async () => (await Filing.fromId(1721696)).close()),
  );
  assert.equal(left.timers, 0);
  // Aborted by the caller or timed out: released too.
  const controller = new AbortController();
  const aborted = await leftovers(12_345, controller.signal, () =>
    withFetch(hang, async () => {
      const pending = Filing.fromId(1, { signal: controller.signal, timeout: 12_345 });
      controller.abort(new Error("stop"));
      await assert.rejects(pending, { message: "stop" });
    }),
  );
  assert.deepEqual(aborted, { timers: 0, listeners: 0 });
  const { signal } = new AbortController();
  const timedOut = await leftovers(15, signal, () =>
    withFetch(hang, () => assert.rejects(Filing.fromId(1, { signal, timeout: 15 }), { name: "TimeoutError" })),
  );
  assert.deepEqual(timedOut, { timers: 0, listeners: 0 });
});

test("fromId: proxy as a prefix, a {url} template or a function", async () => {
  const calls = await withFetch(ok, async () => {
    await Filing.fromId(1721696, { proxy: "https://p.example/?url=" });
    await Filing.fromId(1721696, { proxy: "https://p.example/get/{url}/raw" });
    await Filing.fromId(1721696, { proxy: (u) => `https://p.example/x?${u.slice(-11)}` });
  });
  const encoded = encodeURIComponent(URL_1721696);
  assert.deepEqual(
    calls.map((c) => c.url),
    [`https://p.example/?url=${encoded}`, `https://p.example/get/${encoded}/raw`, "https://p.example/x?1721696.fec"],
  );
});

test("fromId: in a browser, a failed request without a proxy explains CORS", async () => {
  const failed = () => Promise.reject(new TypeError("Failed to fetch"));
  // Outside a browser the fetch error passes through as is.
  await withFetch(failed, async () => {
    await assert.rejects(Filing.fromId(1), { message: "Failed to fetch" });
  });
  await inBrowser(async () => {
    const calls = await withFetch(failed, async () => {
      if ("Deno" in globalThis) {
        // Deno has no CORS: never the hint.
        await assert.rejects(Filing.fromId(1), { message: "Failed to fetch" });
      } else {
        await assert.rejects(Filing.fromId(1), (e) =>
          e instanceof TypeError && e.code === "ERR_CORS" && /CORS/.test(e.message) && e.cause?.message === "Failed to fetch");
        // rss.fetch shares the helper.
        await assert.rejects(rss.fetch(), { name: "TypeError", code: "ERR_CORS" });
      }
      // With a proxy, the error is the proxy's to explain.
      await assert.rejects(Filing.fromId(1, { proxy: "https://p.example/?u=" }), { message: "Failed to fetch" });
    });
    // No User-Agent from a browser (it would be ignored or preflighted).
    if (!("Deno" in globalThis)) assert.deepEqual(calls[0].init.headers, {});
  });
});

test("fromId: in a browser, works through a proxy", async () => {
  await inBrowser(async () => {
    const calls = await withFetch(ok, async () => {
      const filing = await Filing.fromId("FEC-1721696", { proxy: "/proxy?url=" });
      assert.equal(filing.id, "1721696");
      filing.close();
    });
    assert.equal(calls[0].url, `/proxy?url=${encodeURIComponent(URL_1721696)}`);
  });
});

test("fromId: the download is read once, in as many passes as a path", async () => {
  let filing;
  const calls = await withFetch(ok, async () => {
    filing = await Filing.fromId(1721696);
  });
  const path = new Filing(fixture("1721696.fec"));
  const json = (it) => [...it].map((x) => JSON.parse(JSON.stringify(x)));
  const records = json(path.itemizationRecords());
  assert.deepEqual(json(filing.itemizationRecords()), records);
  assert.deepEqual(json(filing.itemizationRecords()), records);
  assert.deepEqual(json(filing.itemizations()), json(path.itemizations()));
  assert.deepEqual(filing.cover, path.cover);
  assert.deepEqual(filing.coverRecord.toJSON(), path.coverRecord.toJSON());
  assert.equal(calls.length, 1);
  filing.close();
  path.close();
});
