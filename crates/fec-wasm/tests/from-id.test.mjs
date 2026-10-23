// The browser build's Filing.fromId(), against a stubbed globalThis.fetch.
// crates/fec-node/tests/from-id.test.mjs covers the shared JS in full; this
// checks the wasm build ships it and that the browser-only path works.
import { test } from "node:test";
import assert from "node:assert/strict";
import { bytes, pyFixture, wasm } from "./helpers.mjs";

const PAC = bytes(pyFixture("1721696.fec"));
const URL_1721696 = "https://docquery.fec.gov/dcdev/posted/1721696.fec";

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

/** Run `body` as if in a browser page. */
async function inBrowser(body) {
  globalThis.document = {};
  try {
    await body();
  } finally {
    delete globalThis.document;
  }
}

test("the browser build exports Filing.fromId", () => {
  assert.equal(typeof wasm.Filing.fromId, "function");
});

test("fromId: a download opens as a wasm Filing with id set", async () => {
  const calls = await withFetch(() => new Response(PAC), async () => {
    const filing = await wasm.Filing.fromId("FEC-1721696");
    assert.equal(filing.id, "1721696");
    assert.equal(filing.filerName, "PFIZER INC. PAC");
    let n = 0;
    for (const _ of filing.itemizations()) n++;
    assert.equal(n, 1387);
    filing.close();
  });
  assert.equal(calls[0].url, URL_1721696);
});

test("fromId: in a browser, CORS error without a proxy; works with one", { skip: "Deno" in globalThis ? "Deno has no CORS" : false }, async () => {
  await inBrowser(async () => {
    await withFetch(() => Promise.reject(new TypeError("Failed to fetch")), async () => {
      await assert.rejects(wasm.Filing.fromId(1721696), (e) =>
        e instanceof TypeError && e.code === "ERR_CORS" && e.cause?.message === "Failed to fetch");
    });
    const calls = await withFetch(() => new Response(PAC), async () => {
      (await wasm.Filing.fromId(1721696, { proxy: "/proxy?url=" })).close();
    });
    assert.equal(calls[0].url, `/proxy?url=${encodeURIComponent(URL_1721696)}`);
    assert.deepEqual(calls[0].init.headers, {});
  });
});

test("fromId: 404 is ERR_FILING_NOT_FOUND", async () => {
  await withFetch(() => new Response("", { status: 404 }), async () => {
    await assert.rejects(wasm.Filing.fromId(1), (e) => e instanceof wasm.FecError && e.code === "ERR_FILING_NOT_FOUND" && e.status === 404);
  });
});
