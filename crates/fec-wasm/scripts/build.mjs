// Build the browser package: wasm (cargo + wasm-bindgen + wasm-opt) into
// wasm/, then the shared TypeScript layer with js/native.ts swapped in, into
// dist/.
//
//   node scripts/build.mjs           # optimized (profile wasm-release + wasm-opt -O3)
//   node scripts/build.mjs --dev     # unoptimized wasm, no wasm-opt (fast rebuilds)
//   node scripts/build.mjs --js      # only the TypeScript step
import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, renameSync, rmSync, statSync, writeFileSync } from "node:fs";
import { jsSourceHash } from "./source-hash.mjs";
import { fileURLToPath, pathToFileURL } from "node:url";
import { join } from "node:path";

const ROOT = fileURLToPath(new URL("..", import.meta.url));
const WORKSPACE = join(ROOT, "..", "..");
const dev = process.argv.includes("--dev");
const jsOnly = process.argv.includes("--js");

function run(cmd, args, opts = {}) {
  console.log(`$ ${cmd} ${args.join(" ")}`);
  const r = spawnSync(cmd, args, { stdio: "inherit", cwd: ROOT, shell: process.platform === "win32", ...opts });
  if (r.error) throw r.error;
  if (r.status !== 0) process.exit(r.status ?? 1);
}

function which(cmd) {
  const r = spawnSync(cmd, ["--version"], { stdio: "ignore", shell: process.platform === "win32" });
  return !r.error && r.status === 0;
}

if (!jsOnly) {
  const profile = dev ? "dev" : "wasm-release";
  run("cargo", ["build", "-p", "fec-wasm", "--target", "wasm32-unknown-unknown", "--profile", profile]);
  const targetDir = process.env.CARGO_TARGET_DIR ?? join(WORKSPACE, "target");
  const wasm = join(targetDir, "wasm32-unknown-unknown", dev ? "debug" : profile, "fec_wasm.wasm");
  rmSync(join(ROOT, "wasm"), { recursive: true, force: true });
  run("wasm-bindgen", ["--target", "web", "--out-dir", "wasm", "--out-name", "libfec", wasm]);
  const out = join(ROOT, "wasm", "libfec_bg.wasm");
  if (dev) {
    // skip wasm-opt
  } else if (which("wasm-opt")) {
    // The features rustc enables by default for wasm32-unknown-unknown.
    run("wasm-opt", [
      "-O3",
      "--enable-bulk-memory",
      "--enable-nontrapping-float-to-int",
      "--enable-sign-ext",
      "--enable-mutable-globals",
      "--enable-reference-types",
      "--enable-multivalue",
      out,
      "-o",
      out,
    ]);
  } else {
    console.warn("wasm-opt not found (binaryen); the wasm is unoptimized by it");
  }
  console.log(`wasm/libfec_bg.wasm: ${(statSync(out).size / 1024).toFixed(0)} KiB`);
  // What this wasm is, for build-browser.mjs's checks before packing.
  writeFileSync(join(ROOT, "wasm", "build.json"), JSON.stringify({ release: !dev }) + "\n");
}

// The shared JS layer, with this package's js/ on top: native.ts in place of
// the napi one, and the entry points (index.ts, manual.ts, workerd.ts) around
// the shared index.ts, which becomes api.ts.
const src = join(ROOT, "build", "src");
rmSync(join(ROOT, "build"), { recursive: true, force: true });
mkdirSync(src, { recursive: true });
cpSync(join(WORKSPACE, "crates", "fec-node", "js"), src, { recursive: true });
renameSync(join(src, "index.ts"), join(src, "api.ts"));
cpSync(join(ROOT, "js"), src, { recursive: true });
if (!existsSync(join(ROOT, "wasm", "libfec.js"))) {
  console.error("wasm/ is missing: run without --js first");
  process.exit(1);
}
// The crate version, read from the wasm here so `version` needs no wasm
// loaded at run time (the shared api.ts reads it on import).
const glue = await import(pathToFileURL(join(ROOT, "wasm", "libfec.js")).href);
glue.initSync({ module: readFileSync(join(ROOT, "wasm", "libfec_bg.wasm")) });
writeFileSync(join(src, "version.ts"), `export const VERSION = ${JSON.stringify(glue.version())};\n`);
// native.ts imports "../wasm/libfec.js": the same relative path from
// build/src (for tsc) as from dist/ (at runtime).
cpSync(join(ROOT, "wasm"), join(ROOT, "build", "wasm"), { recursive: true });
rmSync(join(ROOT, "dist"), { recursive: true, force: true });
run(process.execPath, [join(ROOT, "node_modules", "typescript", "bin", "tsc"), "-p", "tsconfig.json"]);
// The JS sources dist/ was compiled from, so a stale dist/ can be caught.
writeFileSync(join(ROOT, "dist", "build.json"), JSON.stringify({ jsSourceHash: jsSourceHash() }) + "\n");
