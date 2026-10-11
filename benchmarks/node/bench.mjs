// Benchmark @asg017/libfec on one filing under node, deno and bun.
//
//   node benchmarks/node/bench.mjs [--runtime node|deno|bun|all] [file]
//
// Each scenario runs in its own child process (the given runtime), under
// /usr/bin/time for peak RSS. Uses the built package (crates/fec-node/dist;
// `make build-release` there for real numbers). Default file:
// benchmarks/1805248.fec (91 MB, local only).
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const PKG = new URL("../../crates/fec-node/dist/index.js", import.meta.url);
const DEFAULT_FILE = fileURLToPath(new URL("../1805248.fec", import.meta.url));

const SCENARIOS = {
  itemizations: (lib, p) => {
    const f = lib.open(p);
    let n = 0;
    for (const _ of f.itemizations()) n++;
    f.close();
    return n;
  },
  itemizations_touch: (lib, p) => {
    const f = lib.open(p);
    let n = 0;
    let sum = 0;
    for (const row of f.itemizations()) {
      n++;
      switch (row.itemization.type) {
        case "ScheduleA": sum += row.itemization.contribution_amount; break;
        case "ScheduleB": sum += row.itemization.expenditure_amount; break;
        default: sum += row.itemization.form_type.length;
      }
    }
    f.close();
    return sum > 0 ? n : -1;
  },
  rows: (lib, p) => {
    const f = lib.open(p);
    let n = 0;
    for (const _ of f.rows()) n++;
    f.close();
    return n;
  },
  rows_bytes: (lib, p) => {
    const f = lib.open(new Uint8Array(readFileSync(p)));
    let n = 0;
    for (const _ of f.rows()) n++;
    f.close();
    return n;
  },
  records: (lib, p) => {
    const f = lib.open(p);
    let n = 0;
    for (const _ of f.records()) n++;
    f.close();
    return n;
  },
  read: (lib, p) => lib.read(p).rows.length,
  read_itemizations: (lib, p) => lib.read(p).itemizations().length,
  read_json: (lib, p) => {
    const f = lib.open(p);
    let bytes = 0;
    let n = 0;
    for (const row of f.itemizations()) {
      bytes += JSON.stringify(row).length;
      n++;
    }
    f.close();
    return bytes > 0 ? n : -1;
  },
};

const args = process.argv.slice(2);
const flag = (name, fallback) => {
  const i = args.indexOf(name);
  if (i < 0) return fallback;
  const [v] = args.splice(i, 2).slice(1);
  return v;
};

const scenario = flag("--scenario", null);
const runtime = flag("--runtime", "all");
const file = args[0] ?? DEFAULT_FILE;

if (scenario !== null) {
  // Child: run one scenario, print {rows, seconds}.
  const lib = await import(PKG.href);
  const t0 = performance.now();
  const rows = SCENARIOS[scenario](lib, file);
  const seconds = (performance.now() - t0) / 1000;
  console.log(JSON.stringify({ rows, seconds }));
} else {
  const self = fileURLToPath(import.meta.url);
  const runtimes = runtime === "all" ? ["node", "deno", "bun"] : [runtime];
  const linux = process.platform === "linux";
  for (const rt of runtimes) {
    const version = spawnSync(rt, ["--version"], { encoding: "utf8" }).stdout.trim().split("\n")[0];
    console.log(`\n${rt} ${version} — ${file}\n`);
    console.log("| scenario | rows | seconds | peak MB |\n|---|---:|---:|---:|");
    for (const name of Object.keys(SCENARIOS)) {
      const cmd = rt === "deno" ? ["deno", "run", "-A", self] : [rt, self];
      const res = spawnSync("/usr/bin/time", [linux ? "-v" : "-l", ...cmd, "--scenario", name, file], {
        encoding: "utf8",
      });
      if (res.status !== 0) {
        console.log(`| ${name} | failed | | |`);
        console.error(res.stderr);
        continue;
      }
      const { rows, seconds } = JSON.parse(res.stdout.trim().split("\n").pop());
      const m = linux
        ? /Maximum resident set size \(kbytes\): (\d+)/.exec(res.stderr)
        : /(\d+)\s+maximum resident set size/.exec(res.stderr);
      const mb = m ? Number(m[1]) / (linux ? 1024 : 1024 * 1024) : NaN;
      console.log(`| ${name} | ${rows} | ${seconds.toFixed(2)} | ${mb.toFixed(0)} |`);
    }
  }
}
