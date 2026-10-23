// HTTP helpers shared by `rss.fetch` and `Filing.fromId`: both fetch from
// fec.gov hosts that send no CORS headers, so a browser needs a proxy.

/**
 * How to route a request through a proxy, for a browser page: the FEC sends
 * no CORS headers. Not needed in Node, Deno or Bun.
 *
 * - a string containing `{url}`: that placeholder is replaced by the
 *   URL-encoded target URL (`"https://proxy.example/fetch?u={url}"`);
 * - any other string: a prefix for the URL-encoded target URL
 *   (`"https://proxy.example/?url="`);
 * - a function: given the target URL, returns the URL to fetch.
 */
export type ProxyOption = string | ((url: string) => string);

/** The URL to request for `url`, given the `proxy` option. @internal */
export function proxied(url: string, proxy: ProxyOption | undefined): string {
  if (proxy === undefined) return url;
  if (typeof proxy === "function") return proxy(url);
  if (typeof proxy !== "string") throw new TypeError(`proxy must be a string or a function, got ${typeof proxy}`);
  const encoded = encodeURIComponent(url);
  return proxy.includes("{url}") ? proxy.replaceAll("{url}", encoded) : proxy + encoded;
}

/**
 * A browser page or worker, where cross-origin requests need CORS. (Deno
 * defines `WorkerGlobalScope` in its workers, but has no CORS.) @internal
 */
export function inBrowser(): boolean {
  return ("document" in globalThis || "WorkerGlobalScope" in globalThis) && !("Deno" in globalThis);
}

/**
 * `globalThis.fetch(url, init)`, except that in a browser, with no proxy, the
 * bare "Failed to fetch" `TypeError` of a CORS block becomes a `TypeError`
 * with `code: "ERR_CORS"` that says why (the original error is its `cause`).
 * @internal
 */
export async function fetchExplained(
  url: string,
  init: RequestInit,
  proxy: ProxyOption | undefined,
): Promise<Response> {
  try {
    return await globalThis.fetch(url, init);
  } catch (e) {
    if (e instanceof TypeError && proxy === undefined && inBrowser()) {
      throw Object.assign(
        new TypeError(
          `fetching ${url} failed, probably because the FEC sends no CORS headers and the browser ` +
            "blocked the response. Pass a `proxy` option that adds them. The original error is the `cause`.",
          { cause: e },
        ),
        { code: "ERR_CORS" },
      );
    }
    throw e;
  }
}

/**
 * A signal that aborts when `signal` does or after `ms` milliseconds
 * (`Infinity`: never). `undefined` if neither applies. @internal
 */
export function withTimeout(signal: AbortSignal | undefined, ms: number): AbortSignal | undefined {
  const timeout = Number.isFinite(ms) ? AbortSignal.timeout(ms) : undefined;
  if (signal === undefined || timeout === undefined) return signal ?? timeout;
  if (typeof AbortSignal.any === "function") return AbortSignal.any([signal, timeout]);
  // Pre-`AbortSignal.any` runtimes.
  const controller = new AbortController();
  for (const s of [signal, timeout]) {
    if (s.aborted) {
      controller.abort(s.reason);
      break;
    }
    s.addEventListener("abort", () => controller.abort(s.reason), { once: true });
  }
  return controller.signal;
}
