// Benchmark @asg017/libfec on one filing under node, deno and bun.
//
//   node benchmarks/node/bench.mjs [--runtime node|deno|bun|all] [--pkg node|wasm] [file]
//
// Each scenario runs in its own child process (the given runtime), under
// /usr/bin/time for peak RSS. Uses the built package: `--pkg node` (default)
// is crates/fec-node/dist (`make build-release` there for real numbers),
// `--pkg wasm` the browser build in crates/fec-wasm/dist (`make build`
// there), which takes bytes only, so its path scenarios read the file first
// (timed). Default file: benchmarks/1805248.fec (91 MB, local only).
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const PKGS = {
  node: new URL("../../crates/fec-node/dist/index.js", import.meta.url),
  wasm: new URL("../../crates/fec-wasm/dist/index.js", import.meta.url),
};

/** The wasm build with paths read into bytes, so every scenario runs unchanged. */
function bytesOnly(lib) {
  const bytes = (p) => (typeof p === "string" ? new Uint8Array(readFileSync(p)) : p);
  return { ...lib, open: (p, o) => lib.open(bytes(p), o), read: (p, o) => lib.read(bytes(p), o) };
}
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
const pkg = flag("--pkg", "node");
if (!(pkg in PKGS)) throw new Error(`--pkg must be node or wasm, got ${pkg}`);
const file = args[0] ?? DEFAULT_FILE;

if (scenario !== null) {
  // Child: run one scenario, print {rows, seconds}.
  const mod = await import(PKGS[pkg].href);
  const lib = pkg === "wasm" ? bytesOnly(mod) : mod;
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
    console.log(`\n${rt} ${version} — ${pkg} build — ${file}\n`);
    console.log("| scenario | rows | seconds | peak MB |\n|---|---:|---:|---:|");
    for (const name of Object.keys(SCENARIOS)) {
      const cmd = rt === "deno" ? ["deno", "run", "-A", self] : [rt, self];
      const res = spawnSync("/usr/bin/time", [linux ? "-v" : "-l", ...cmd, "--scenario", name, "--pkg", pkg, file], {
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
