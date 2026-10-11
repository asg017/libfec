// Copy the browser build (crates/fec-wasm: `make build` there) into
// browser/, which package.json's "browser" export condition points at, so
// bundlers resolve `@asg017/libfec` to the wasm build for the web. The same
// files are published on their own as @asg017/libfec-wasm.
import { cpSync, existsSync, rmSync } from "node:fs";

const WASM = new URL("../../fec-wasm/", import.meta.url);
const OUT = new URL("../browser/", import.meta.url);

for (const dir of ["dist", "wasm"]) {
  if (!existsSync(new URL(dir, WASM))) {
    console.error(`crates/fec-wasm/${dir} is missing: run \`make build\` in crates/fec-wasm first`);
    process.exit(1);
  }
}
rmSync(OUT, { recursive: true, force: true });
cpSync(new URL("dist", WASM), new URL("dist", OUT), { recursive: true });
for (const f of ["libfec.js", "libfec.d.ts", "libfec_bg.wasm"]) {
  cpSync(new URL(`wasm/${f}`, WASM), new URL(`wasm/${f}`, OUT));
}
console.log("browser/: dist + wasm copied from crates/fec-wasm");
