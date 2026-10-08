// Run every examples/*.mjs under one runtime; any failure fails the run.
//   node scripts/examples.mjs node|deno|bun
import { spawnSync } from "node:child_process";
import { readdirSync } from "node:fs";

const runtime = process.argv[2];
const prefix = {
  node: ["node"],
  bun: ["bun"],
  deno: ["deno", "run", "--allow-read", "--allow-ffi", "--allow-env"],
}[runtime];
if (!prefix) {
  console.error("usage: node scripts/examples.mjs node|deno|bun");
  process.exit(2);
}
const examples = readdirSync(new URL("../examples/", import.meta.url))
  .filter((f) => f.endsWith(".mjs") && !f.startsWith("_"))
  .sort();
let failed = 0;
for (const f of examples) {
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
