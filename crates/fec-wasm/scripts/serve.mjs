// A small static server for the demo and the browser tests, with the two
// things a plain `python -m http.server` lacks:
//
// - /proxy?url=<encoded>: fetches an FEC URL server-side and adds CORS
//   headers (the FEC sends none, so a page can't fetch the RSS feed or a
//   .fec file directly). Only efilingapps.fec.gov and docquery.fec.gov.
// - --csp strict: every page and script (workers too: a worker takes its CSP
//   from its own script's response) under a CSP without 'unsafe-eval' (only
//   'wasm-unsafe-eval'), as strict sites do.
//
// Development only: it listens on 127.0.0.1 and answers only requests whose
// Host is 127.0.0.1 or localhost (no DNS rebinding).
//
//   node scripts/serve.mjs [--port 8000] [--root ../] [--feed saved.xml] [--csp strict]
//
// --root defaults to crates/, so pages can import ../fec-wasm/dist and read
// the committed fixtures. --feed answers every feed request from a saved
// file (offline tests). The module also exports `serve()` for the tests.
import { createReadStream, readFileSync, statSync } from "node:fs";
import { createServer } from "node:http";
import { Readable } from "node:stream";
import { extname, join, normalize, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json",
  ".wasm": "application/wasm",
  ".map": "application/json",
  ".ts": "text/plain; charset=utf-8",
  ".xml": "application/xml",
  ".svg": "image/svg+xml",
  ".fec": "application/octet-stream",
};
const PROXY_HOSTS = new Set(["efilingapps.fec.gov", "docquery.fec.gov"]);
const STRICT_CSP = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; worker-src 'self'";

/**
 * Start the server. Resolves to `{ url, close }`.
 *
 * `files` maps extra URL paths to files outside `root` (the benchmark filing).
 *
 * @param {{ port?: number, root?: string, feed?: string, files?: Record<string, string>, csp?: "strict", log?: boolean }} options
 */
export function serve({ port = 0, root = fileURLToPath(new URL("../../", import.meta.url)), feed, files = {}, csp, log = false } = {}) {
  root = resolve(root);
  const savedFeed = feed ? readFileSync(feed) : null;
  const server = createServer(async (req, res) => {
    const url = new URL(req.url ?? "/", "http://localhost");
    if (log) console.log(req.method, url.pathname + url.search);
    const { port } = server.address();
    if (req.headers.host !== `127.0.0.1:${port}` && req.headers.host !== `localhost:${port}`) {
      return send(res, 421, "unexpected Host");
    }
    try {
      if (url.pathname === "/proxy") return await proxy(url, res, savedFeed);
      let path = files[url.pathname] ?? normalize(join(root, decodeURIComponent(url.pathname)));
      if (!files[url.pathname] && path !== root && !path.startsWith(root + sep)) return send(res, 403, "forbidden");
      let stat = statSync(path, { throwIfNoEntry: false });
      if (stat?.isDirectory()) {
        path = join(path, "index.html");
        stat = statSync(path, { throwIfNoEntry: false });
      }
      if (!stat?.isFile()) return send(res, 404, "not found");
      const headers = {
        "content-type": TYPES[extname(path)] ?? "application/octet-stream",
        "content-length": stat.size,
        "cache-control": "no-store",
      };
      if (csp === "strict" && /\.(html|m?js)$/.test(path)) headers["content-security-policy"] = STRICT_CSP;
      res.writeHead(200, headers);
      createReadStream(path).pipe(res);
    } catch (e) {
      send(res, 500, String(e));
    }
  });
  return new Promise((ok) =>
    server.listen(port, "127.0.0.1", () => {
      const { port } = server.address();
      ok({ url: `http://127.0.0.1:${port}`, close: () => new Promise((done) => server.close(done)) });
    }),
  );
}

function send(res, status, body) {
  res.writeHead(status, { "content-type": "text/plain; charset=utf-8", "access-control-allow-origin": "*" });
  res.end(body);
}

/** An FEC URL: https or http, an allowed host, the default port. */
function allowed(target) {
  return (target.protocol === "https:" || target.protocol === "http:") && PROXY_HOSTS.has(target.hostname) && target.port === "";
}

async function proxy(url, res, savedFeed) {
  let target;
  try {
    target = new URL(url.searchParams.get("url") ?? "");
  } catch {
    return send(res, 400, "bad url");
  }
  if (!allowed(target)) return send(res, 403, `not an allowed FEC URL: ${target.href}`);
  const cors = { "access-control-allow-origin": "*", "cache-control": "no-store" };
  if (savedFeed && target.hostname === "efilingapps.fec.gov") {
    res.writeHead(200, { ...cors, "content-type": "application/xml" });
    return res.end(savedFeed);
  }
  // Follow redirects by hand, so each hop is checked against the allowlist.
  let upstream;
  for (let hops = 0; ; hops++) {
    upstream = await fetch(target, { redirect: "manual", headers: { "user-agent": "libfec-wasm demo proxy" } });
    const location = upstream.headers.get("location");
    if (upstream.status < 300 || upstream.status >= 400 || !location) break;
    if (hops === 5) return send(res, 508, "too many redirects");
    target = new URL(location, target);
    if (!allowed(target)) return send(res, 403, `redirected off the allowed hosts: ${target.href}`);
  }
  const headers = { ...cors, "content-type": upstream.headers.get("content-type") ?? "application/octet-stream" };
  // fetch() decompresses, so a compressed response's length isn't the body's.
  const length = upstream.headers.get("content-length");
  if (length && !upstream.headers.get("content-encoding")) headers["content-length"] = length;
  res.writeHead(upstream.status, headers);
  if (upstream.body) Readable.fromWeb(upstream.body).pipe(res);
  else res.end();
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  const flag = (name) => {
    const i = args.indexOf(name);
    return i < 0 ? undefined : args[i + 1];
  };
  const { url } = await serve({
    port: Number(flag("--port") ?? 8000),
    root: flag("--root"),
    feed: flag("--feed"),
    csp: flag("--csp"),
    log: true,
  });
  console.log(`serving on ${url}/fec-wasm/demo/`);
}
