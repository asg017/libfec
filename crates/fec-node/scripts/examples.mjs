// Run every examples/*.mjs under one runtime; any failure fails the run.
//   node scripts/examples.mjs node|deno|bun [--network]
// An example whose first line starts with `// @network` makes live requests:
// it is skipped unless `--network` is given.
import { spawnSync } from "node:child_process";
import { readFileSync, readdirSync } from "node:fs";

const runtime = process.argv[2];
const network = process.argv.includes("--network");
const prefix = {
  node: ["node"],
  bun: ["bun"],
  deno: ["deno", "run", "--allow-read", "--allow-ffi", "--allow-env", ...(network ? ["--allow-net"] : [])],
}[runtime];
if (!prefix) {
  console.error("usage: node scripts/examples.mjs node|deno|bun [--network]");
  process.exit(2);
}
const dir = new URL("../examples/", import.meta.url);
const examples = readdirSync(dir)
  .filter((f) => f.endsWith(".mjs") && !f.startsWith("_"))
  .sort();
const needsNetwork = (/** @type {string} */ f) => readFileSync(new URL(f, dir), "utf8").startsWith("// @network");
let failed = 0;
for (const f of examples) {
  if (needsNetwork(f) && !network) {
    console.log(`skip ${runtime} examples/${f} (network; pass --network)`);
    continue;
  }
  const [cmd, ...args] = prefix;
  const res = spawnSync(cmd, [...args, `examples/${f}`], {
    encoding: "utf8",
    shell: process.platform === "win32",
  });
  const ok = res.status === 0;
  if (!ok) failed++;
  console.log(`${ok ? "ok  " : "FAIL"} ${runtime} examples/${f}`);
  if (!ok) console.log(res.stdout + res.stderr);
}
process.exit(failed ? 1 : 0);
