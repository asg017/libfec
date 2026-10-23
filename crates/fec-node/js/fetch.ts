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

/** A request's abort signal, and `cleanup()` to call when the request is done. @internal */
export interface Deadline {
  /** Aborts when the caller's signal does or the time is up; `undefined` if neither applies. */
  signal: AbortSignal | undefined;
  /** Clear the timer and remove the listener on the caller's signal. Idempotent. */
  cleanup(): void;
}

/**
 * A signal that aborts when `signal` does or after `ms` milliseconds
 * (`Infinity`: never), with a `TimeoutError` like `AbortSignal.timeout()`'s.
 *
 * A cancellable timer rather than `AbortSignal.timeout()`, which can't be
 * cleared (Deno keeps the process alive until it fires), and one listener on
 * the caller's signal rather than `AbortSignal.any()`, which older runtimes
 * lack. Call `cleanup()` once the response has been read, on every path, or
 * a long-lived caller's signal collects a listener per request. @internal
 */
export function withTimeout(signal: AbortSignal | undefined, ms: number): Deadline {
  if (!Number.isFinite(ms)) return { signal, cleanup: () => {} };
  const controller = new AbortController();
  const onAbort = () => controller.abort(signal!.reason);
  const timer = setTimeout(
    () => controller.abort(new DOMException("The operation was aborted due to timeout", "TimeoutError")),
    ms,
  );
  const cleanup = () => {
    clearTimeout(timer);
    signal?.removeEventListener("abort", onAbort);
  };
  // Aborted either way: nothing left to wait for.
  controller.signal.addEventListener("abort", cleanup, { once: true });
  if (signal?.aborted) onAbort();
  else signal?.addEventListener("abort", onAbort, { once: true });
  return { signal: controller.signal, cleanup };
}
