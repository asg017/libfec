// A small static server for the demo and the browser tests, with the two
// things a plain `python -m http.server` lacks:
//
// - /proxy?url=<encoded>: fetches an FEC URL server-side and adds CORS
//   headers (the FEC sends none, so a page can't fetch the RSS feed or a
//   .fec file directly). Only efilingapps.fec.gov and docquery.fec.gov.
// - ?csp=strict on an .html page: serves it under a CSP without
//   'unsafe-eval' (only 'wasm-unsafe-eval'), as strict sites do.
//
//   node scripts/serve.mjs [--port 8000] [--root ../] [--feed saved.xml]
//
// --root defaults to crates/, so pages can import ../fec-wasm/dist and read
// the committed fixtures. --feed answers every feed request from a saved
// file (offline tests). The module also exports `serve()` for the tests.
import { createReadStream, readFileSync, statSync } from "node:fs";
import { createServer } from "node:http";
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
 * @param {{ port?: number, root?: string, feed?: string, files?: Record<string, string>, log?: boolean }} options
 */
export function serve({ port = 0, root = fileURLToPath(new URL("../../", import.meta.url)), feed, files = {}, log = false } = {}) {
  root = resolve(root);
  const savedFeed = feed ? readFileSync(feed) : null;
  const server = createServer(async (req, res) => {
    const url = new URL(req.url ?? "/", "http://localhost");
    if (log) console.log(req.method, url.pathname + url.search);
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
      if (url.searchParams.get("csp") === "strict" && path.endsWith(".html")) {
        headers["content-security-policy"] = STRICT_CSP;
      }
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

async function proxy(url, res, savedFeed) {
  const target = new URL(url.searchParams.get("url") ?? "");
  if (target.protocol !== "https:" && target.protocol !== "http:") return send(res, 400, "bad url");
  if (!PROXY_HOSTS.has(target.hostname)) return send(res, 403, `not an FEC host: ${target.hostname}`);
  const cors = { "access-control-allow-origin": "*", "cache-control": "no-store" };
  if (savedFeed && target.hostname === "efilingapps.fec.gov") {
    res.writeHead(200, { ...cors, "content-type": "application/xml" });
    return res.end(savedFeed);
  }
  const upstream = await fetch(target, { headers: { "user-agent": "libfec-wasm demo proxy" } });
  res.writeHead(upstream.status, { ...cors, "content-type": upstream.headers.get("content-type") ?? "application/octet-stream" });
  res.end(Buffer.from(await upstream.arrayBuffer()));
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
    log: true,
  });
  console.log(`serving on ${url}/fec-wasm/demo/`);
}
