// Copy the browser build (crates/fec-wasm: `make build` there) into
// browser/, which package.json's "browser" export condition points at, so
// bundlers resolve `@asg017/libfec` to the wasm build for the web. The same
// files are published on their own as @asg017/libfec-wasm.
//
// Also the package's `prepack`: npm silently leaves out missing "files"
// entries, so packing without the browser build would publish a "browser"
// condition that points nowhere. This fails instead, and also refuses an
// unoptimized (`--dev`) wasm or a dist/ compiled from older js/ sources.
//   node scripts/build-browser.mjs [--allow-dev]
import { cpSync, existsSync, readFileSync, rmSync } from "node:fs";
import { jsSourceHash } from "../../fec-wasm/scripts/source-hash.mjs";

const WASM = new URL("../../fec-wasm/", import.meta.url);
const OUT = new URL("../browser/", import.meta.url);
const allowDev = process.argv.includes("--allow-dev");

function fail(message) {
  console.error(`build-browser: ${message}`);
  process.exit(1);
}

const read = (path) => {
  const url = new URL(path, WASM);
  if (!existsSync(url)) fail(`crates/fec-wasm/${path} is missing: run \`make build\` in crates/fec-wasm first`);
  return JSON.parse(readFileSync(url, "utf8"));
};
if (!read("wasm/build.json").release && !allowDev) {
  fail("crates/fec-wasm/wasm is a --dev build (unoptimized): run `make build` in crates/fec-wasm");
}
if (read("dist/build.json").jsSourceHash !== jsSourceHash()) {
  fail("crates/fec-wasm/dist is stale (js/ changed since it was built): run `make build-js` in crates/fec-wasm");
}

rmSync(OUT, { recursive: true, force: true });
cpSync(new URL("dist", WASM), new URL("dist", OUT), { recursive: true });
for (const f of ["libfec.js", "libfec.d.ts", "libfec_bg.wasm"]) {
  cpSync(new URL(`wasm/${f}`, WASM), new URL(`wasm/${f}`, OUT));
}
console.log("browser/: dist + wasm copied from crates/fec-wasm");
