// The wasm memory-per-pass check, shared by tests/memory.test.mjs (Node,
// Deno, Bun) and the browser harness (page.mjs): a filing's bytes are copied
// into wasm memory once, by `new Filing()`, and every pass reads that copy
// (crates/fec-wasm/src/reader.rs, Rc<[u8]>), so the heap must not grow by the
// file's size per pass. No Node or DOM APIs here.

/** `head` (header and cover lines) then `body` (the rest) repeated to at least `mb` MB. */
export function bigFiling(fec, mb) {
  const text = new TextDecoder("latin1").decode(fec);
  const lines = text.split("\n");
  const head = lines.slice(0, 2).join("\n") + "\n";
  const bodyLines = lines.slice(2).filter((l) => l !== "");
  const body = bodyLines.join("\n") + "\n";
  const copies = Math.ceil((mb * 2 ** 20) / body.length);
  // latin1: one char per byte.
  const enc = (s) => Uint8Array.from(s, (c) => c.charCodeAt(0));
  const h = enc(head);
  const b = enc(body);
  const data = new Uint8Array(h.length + b.length * copies);
  data.set(h);
  for (let i = 0; i < copies; i++) data.set(b, h.length + i * b.length);
  return { data, rows: bodyLines.length * copies };
}

function advance(pass, n) {
  for (let i = 0; i < n; i++) {
    if (pass.next().done) throw new Error("pass ended early");
  }
}

/**
 * Heap growth, in bytes, while a filing of `data` is opened and read in
 * several passes. No pass is drained: a per-pass copy would happen when the
 * pass starts, and a full pass of a big filing takes seconds in a dev build.
 *
 * - `opened`: `new Filing()` (should be about `data.length`: the one copy,
 *   and proof there was no slack in the heap to hide another);
 * - `twoPasses`: a pass, then a second one after it;
 * - `fourLive`: four passes live at once;
 * - `reopened`: another filing of the same size after `close()` (the space
 *   is reused).
 */
export function memoryPerPass(lib, memory, data) {
  const heap = () => memory.buffer.byteLength;
  const before = heap();
  const f = new lib.Filing(data, { batchSize: 256 });
  const opened = heap();
  for (const view of ["itemizationRecords", "itemizations"]) {
    const pass = f[view]();
    advance(pass, 2000);
    pass.return();
  }
  const afterTwo = heap();
  const passes = [f.itemizations(), f.itemizationRecords(), f.itemizations(), f.itemizationRecords()];
  for (const p of passes) advance(p, 1000);
  const live = heap();
  f.close();
  const g = new lib.Filing(data);
  const reopened = heap();
  g.close();
  return {
    size: data.length,
    opened: opened - before,
    twoPasses: afterTwo - opened,
    fourLive: live - afterTwo,
    reopened: reopened - live,
  };
}

/** The assertions on {@link memoryPerPass}'s result; returns a list of failures. */
export function memoryProblems(r) {
  const mb = (n) => `${(n / 2 ** 20).toFixed(1)} MB`;
  const problems = [];
  if (r.opened < r.size * 0.9) problems.push(`open grew the heap by only ${mb(r.opened)} for ${mb(r.size)}: slack could hide a copy`);
  for (const k of ["twoPasses", "fourLive", "reopened"]) {
    if (r[k] >= r.size / 2) problems.push(`${k} grew the heap by ${mb(r[k])} (file: ${mb(r.size)})`);
  }
  return problems;
}
