// Run tests/*.test.mjs under one runtime: `node scripts/test.mjs node|deno|bun [--slow]`.
//
// A script rather than Makefile globs so it runs the same on Windows CI, and
// because each runtime needs a different invocation:
// - node: `using` is a SyntaxError before Node 24, so tests/using.test.mjs is
//   left out there (by file: a parse error can't be caught).
// - deno: the generated napi-rs loader reads NAPI_RS_* env vars, hence --allow-env.
// - bun: Bun 1.2's node:test shim only runs the first file's tests when given
//   several files (the rest pass silently), so each file gets its own `bun test`.
import { spawnSync } from "node:child_process";
import { readdirSync } from "node:fs";

const runtime = process.argv[2];
// `--slow`: only the *.slow.test.mjs files (perf guards on the 91 MB filing).
const slow = process.argv.includes("--slow");
let files = readdirSync(new URL("../tests/", import.meta.url))
  .filter((f) => f.endsWith(".test.mjs") && f.endsWith(".slow.test.mjs") === slow)
  .sort()
  .map((f) => `./tests/${f}`);

const runs = [];
switch (runtime) {
  case "node": {
    const major = Number(process.versions.node.split(".")[0]);
    if (major < 24) files = files.filter((f) => !f.endsWith("/using.test.mjs"));
    runs.push(["node", ["--test", ...files]]);
    break;
  }
  case "deno":
    runs.push(["deno", ["test", "--allow-read", "--allow-ffi", "--allow-env", ...files]]);
    break;
  case "bun":
    for (const f of files) runs.push(["bun", ["test", f]]);
    break;
  default:
    console.error("usage: node scripts/test.mjs node|deno|bun");
    process.exit(2);
}

console.log(`${runtime}: ${files.join(" ")}`);
for (const [cmd, args] of runs) {
  const { status, error } = spawnSync(cmd, args, {
    stdio: "inherit",
    shell: process.platform === "win32",
  });
  if (error) throw error;
  if (status !== 0) process.exit(status ?? 1);
}
