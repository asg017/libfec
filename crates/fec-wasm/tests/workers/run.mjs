// Run the packed @alex.garcia/libfec-wasm in Cloudflare's workerd, as a
// dependent would install it:
//
//   node tests/workers/run.mjs
//
// Packs this package, installs the tarball into a scratch Worker project
// (build/workers/), then for both ways of loading it (default.mjs: the plain
// import under the "workerd" condition; manual.mjs: "./manual" + initSync on
// the package's `.wasm` export):
// - `wrangler deploy --dry-run` must bundle the wasm;
// - under `wrangler dev`, every committed fixture POSTed to the Worker must
//   summarize identically to the napi build in Node.
// wrangler runs through npx, pinned, so it isn't a devDependency of every
// `npm ci` here.
import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { cpSync, mkdirSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:net";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { FIXTURES, bytes, napi } from "../helpers.mjs";
import { summarize } from "../browser/summarize.mjs";

const WRANGLER = "wrangler@4.149.0";
const ROOT = fileURLToPath(new URL("../../", import.meta.url));
const HERE = fileURLToPath(new URL(".", import.meta.url));
const DIR = join(ROOT, "build", "workers");
const shell = process.platform === "win32";

function run(cmd, args, opts = {}) {
  const r = spawnSync(cmd, args, { cwd: DIR, encoding: "utf8", shell, ...opts });
  if (r.error) throw r.error;
  assert.equal(r.status, 0, `${cmd} ${args.join(" ")}\n${r.stdout}\n${r.stderr}`);
  return r.stdout;
}

/** A port nothing listens on. */
function freePort() {
  return new Promise((resolve, reject) => {
    const s = createServer().listen(0, "127.0.0.1", () => {
      const { port } = s.address();
      s.close(() => resolve(port));
    });
    s.on("error", reject);
  });
}

// ---- the scratch project ---------------------------------------------------

rmSync(DIR, { recursive: true, force: true });
mkdirSync(DIR, { recursive: true });
const tgz = run("npm", ["pack", "--silent", "--pack-destination", DIR], { cwd: ROOT }).trim().split("\n").pop();
const pkg = join(DIR, "node_modules", "@alex.garcia", "libfec-wasm");
mkdirSync(pkg, { recursive: true });
run("tar", ["xzf", join(DIR, tgz), "-C", pkg, "--strip-components", "1"]);
for (const f of ["default.mjs", "manual.mjs", "respond.mjs"]) cpSync(join(HERE, f), join(DIR, f));
cpSync(join(HERE, "..", "browser", "summarize.mjs"), join(DIR, "summarize.mjs"));

const env = { ...process.env, WRANGLER_SEND_METRICS: "false", CI: "1" };
const expected = FIXTURES.map(([label, path]) => [label, bytes(path), JSON.parse(JSON.stringify(summarize(napi, bytes(path))))]);

let failed = 0;
for (const entry of ["default", "manual"]) {
  const config = join(DIR, `${entry}.toml`);
  writeFileSync(config, `name = "libfec-${entry}"\nmain = "${entry}.mjs"\ncompatibility_date = "2026-10-01"\n`);

  // The bundle must carry the wasm as a module of its own.
  const out = join(DIR, `out-${entry}`);
  run("npx", ["--yes", WRANGLER, "deploy", "--dry-run", "--config", config, "--outdir", out], { env });
  const wasmFiles = readdirSync(out).filter((f) => f.endsWith(".wasm"));
  assert.equal(wasmFiles.length, 1, `${entry}: the bundle has no .wasm (${readdirSync(out).join(", ")})`);

  const port = await freePort();
  const dev = spawn("npx", ["--yes", WRANGLER, "dev", "--config", config, "--ip", "127.0.0.1", "--port", String(port), "--show-interactive-dev-session=false"], {
    cwd: DIR,
    env,
    shell,
    stdio: ["ignore", "pipe", "pipe"],
  });
  let log = "";
  dev.stdout.on("data", (d) => (log += d));
  dev.stderr.on("data", (d) => (log += d));
  const exited = new Promise((resolve) => dev.on("exit", resolve));
  try {
    const url = `http://127.0.0.1:${port}/`;
    // Up when it answers; a Worker that fails at startup never does.
    let info;
    for (const deadline = Date.now() + 60_000; !info; ) {
      if (dev.exitCode !== null || Date.now() > deadline) throw new Error(`${entry}: wrangler dev didn't start\n${log}`);
      try {
        const r = await fetch(url);
        if (r.ok) info = await r.json();
      } catch {
        await new Promise((r) => setTimeout(r, 250));
      }
    }
    assert.equal(info.version, napi.version, `${entry}: version`);
    for (const [label, data, want] of expected) {
      const got = await (await fetch(url, { method: "POST", body: data })).json();
      try {
        assert.deepEqual(got, want);
      } catch (e) {
        failed++;
        console.error(`✘ ${entry}: ${label}\n${e.message.slice(0, 2000)}`);
      }
    }
    assert.doesNotMatch(log, /\bERROR\b/, `${entry}: wrangler dev logged an error\n${log}`);
    console.log(`✔ ${entry}: ${expected.length} fixtures match the napi build in workerd`);
  } finally {
    dev.kill();
    await exited;
  }
}
if (failed) {
  console.error(`${failed} fixture(s) differ`);
  process.exit(1);
}
