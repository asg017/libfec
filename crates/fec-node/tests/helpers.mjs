import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";

/** The committed Python fixtures, shared by both bindings. */
export const FIXTURES = new URL("../../fec-py/tests/fixtures/", import.meta.url);
/** fec-parser's legacy-format fixtures (1.x–7.0, paper). */
export const LEGACY_FIXTURES = new URL("../../fec-parser/tests/fixtures/legacy/", import.meta.url);

/** Path to a fixture; a missing fixture fails the test, never skips it. */
export function fixture(name, base = FIXTURES) {
  const path = fileURLToPath(new URL(name, base));
  if (!existsSync(path)) throw new Error(`missing fixture: ${path}`);
  return path;
}

/**
 * Run `body` and report what it left behind: the `ms`-millisecond timers
 * still pending (set, neither fired nor cleared), and the abort listeners
 * still on `signal`. Counted by wrapping `setTimeout`/`clearTimeout` and the
 * signal's listener methods, which works the same in Node, Deno and Bun.
 */
export async function leftovers(ms, signal, body) {
  const timers = new Set();
  const listeners = new Set();
  const { setTimeout: set, clearTimeout: clear } = globalThis;
  globalThis.setTimeout = (fn, delay, ...args) => {
    if (delay !== ms) return set(fn, delay, ...args);
    const id = set(() => {
      timers.delete(id);
      fn(...args);
    }, delay);
    timers.add(id);
    return id;
  };
  globalThis.clearTimeout = (id) => {
    timers.delete(id);
    clear(id);
  };
  const { addEventListener: add, removeEventListener: remove } = signal ?? {};
  if (signal) {
    signal.addEventListener = function (type, fn, opts) {
      if (type === "abort") listeners.add(fn);
      return add.call(this, type, fn, opts);
    };
    signal.removeEventListener = function (type, fn, opts) {
      if (type === "abort") listeners.delete(fn);
      return remove.call(this, type, fn, opts);
    };
  }
  try {
    await body();
  } finally {
    globalThis.setTimeout = set;
    globalThis.clearTimeout = clear;
    if (signal) {
      delete signal.addEventListener;
      delete signal.removeEventListener;
    }
  }
  for (const id of timers) clear(id);
  return { timers: timers.size, listeners: listeners.size };
}
