// Regression guards on the 91 MB benchmark filing (`make test-slow`; not in
// `make test`). Looser than the plan's targets (plans/nodejs/00-decisions.md
// T9); benchmarks/node/bench.mjs has the real numbers. Build with
// `make build-release` first, or the debug addon will fail the time guard.
import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { open } from "../dist/index.js";

const FILE =
  process.env.FEC_BENCH_FILE ??
  fileURLToPath(new URL("../../../benchmarks/1805248.fec", import.meta.url));
// The one allowed skip: the file is local-only (gitignored), like Python's
// `benchmark_fec_file` fixture. RSS is sampled in-process under the test
// runner, which adds ~100 MB over bench.mjs's numbers.
const skip = existsSync(FILE) ? false : `no ${FILE} (set FEC_BENCH_FILE)`;

/** Iterate, sampling RSS every 10,000 rows: [rows, seconds, peak MB]. */
function measure(method) {
  const t0 = performance.now();
  let peak = 0;
  let n = 0;
  const f = open(FILE);
  try {
    for (const _ of f[method]()) {
      if (++n % 10000 === 0) peak = Math.max(peak, process.memoryUsage().rss);
    }
  } finally {
    f.close();
  }
  return [n, (performance.now() - t0) / 1000, peak / 2 ** 20];
}

test("rows(): all 408,160 rows, under 1 s and 400 MB", { skip }, () => {
  const [n, s, mb] = measure("rows");
  assert.equal(n, 408160);
  assert.ok(s < 1, `${s.toFixed(2)} s`);
  assert.ok(mb < 400, `${mb.toFixed(0)} MB`);
});

test("itemizations(): all 408,160 rows typed, under 3 s and 600 MB", { skip }, () => {
  const [n, s, mb] = measure("itemizations");
  assert.equal(n, 408160);
  assert.ok(s < 3, `${s.toFixed(2)} s`);
  assert.ok(mb < 600, `${mb.toFixed(0)} MB`);
});
