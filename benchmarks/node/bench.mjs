// Benchmark @alex.garcia/libfec on one filing under node, deno and bun.
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
  const Filing = class extends lib.Filing {
    constructor(p, o) {
      super(bytes(p), o);
    }
  };
  return { ...lib, Filing };
}
const DEFAULT_FILE = fileURLToPath(new URL("../1805248.fec", import.meta.url));

/** Count what `pass(filing)` yields, then close the filing. */
function count(lib, source, pass) {
  const f = new lib.Filing(source);
  try {
    let n = 0;
    for (const _ of pass(f)) n++;
    return n;
  } finally {
    f.close();
  }
}

const SCENARIOS = {
  itemizations: (lib, p) => count(lib, p, (f) => f.itemizations()),
  itemizations_touch: (lib, p) => {
    const f = new lib.Filing(p);
    let n = 0;
    let sum = 0;
    for (const item of f.itemizations()) {
      n++;
      switch (item.type) {
        case "ScheduleA": sum += item.contribution_amount; break;
        case "ScheduleB": sum += item.expenditure_amount; break;
        case "Unknown": sum += item.formType.length; break;
        default: sum += item.form_type.length;
      }
    }
    f.close();
    return sum > 0 ? n : -1;
  },
  itemization_records: (lib, p) => count(lib, p, (f) => f.itemizationRecords()),
  itemization_records_bytes: (lib, p) => count(lib, new Uint8Array(readFileSync(p)), (f) => f.itemizationRecords()),
  itemizations_json: (lib, p) => {
    const f = new lib.Filing(p);
    let bytes = 0;
    let n = 0;
    for (const item of f.itemizations()) {
      bytes += JSON.stringify(item).length;
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
