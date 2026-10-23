// Run tests/*.test.mjs under one runtime: `node scripts/test.mjs node|deno|bun [--slow|--network]`.
//
// A script rather than Makefile globs so it runs the same on Windows CI, and
// because each runtime needs a different invocation:
// - gc() for the GC tests: node --expose-gc (via NODE_OPTIONS), deno --v8-flags=--expose-gc
//   (bun has Bun.gc()).
// - node: `using` is a SyntaxError before Node 24, so tests/using.test.mjs is
//   left out there (by file: a parse error can't be caught).
// - deno: the generated napi-rs loader reads NAPI_RS_* env vars, hence --allow-env.
// - bun: Bun 1.2's node:test shim only runs the first file's tests when given
//   several files (the rest pass silently), so each file gets its own `bun test`.
import { spawnSync } from "node:child_process";
import { readdirSync } from "node:fs";
import { tmpdir } from "node:os";

const runtime = process.argv[2];
// `--slow`: only the *.slow.test.mjs files (perf guards on the 91 MB filing).
// `--network`: only the *.network.test.mjs files (live requests to fec.gov).
// Neither kind runs by default.
const kind = process.argv.includes("--slow") ? "slow" : process.argv.includes("--network") ? "network" : null;
const kindOf = (f) => (f.endsWith(".slow.test.mjs") ? "slow" : f.endsWith(".network.test.mjs") ? "network" : null);
let files = readdirSync(new URL("../tests/", import.meta.url))
  .filter((f) => f.endsWith(".test.mjs") && kindOf(f) === kind)
  .sort()
  .map((f) => `./tests/${f}`);

const runs = [];
switch (runtime) {
  case "node": {
    const major = Number(process.versions.node.split(".")[0]);
    if (major < 24) files = files.filter((f) => !f.endsWith("/using.test.mjs"));
    // In NODE_OPTIONS: `node --test` doesn't pass its own flags to the
    // per-file child processes.
    runs.push(["node", ["--test", ...files], { NODE_OPTIONS: `${process.env.NODE_OPTIONS ?? ""} --expose-gc`.trim() }]);
    break;
  }
  case "deno":
    // --allow-write to the temp dir only: tests copy fixtures there to name them.
    runs.push(["deno", ["test", "--v8-flags=--expose-gc", "--allow-read", "--allow-ffi", "--allow-env", `--allow-write=${tmpdir()}`, ...(kind === "network" ? ["--allow-net"] : []), ...files]]);
    break;
  case "bun":
    for (const f of files) runs.push(["bun", ["test", f]]);
    break;
  default:
    console.error("usage: node scripts/test.mjs node|deno|bun");
    process.exit(2);
}

console.log(`${runtime}: ${files.join(" ")}`);
for (const [cmd, args, env] of runs) {
  const { status, error } = spawnSync(cmd, args, {
    stdio: "inherit",
    env: { ...process.env, ...env },
    shell: process.platform === "win32",
  });
  if (error) throw error;
  if (status !== 0) process.exit(status ?? 1);
}
