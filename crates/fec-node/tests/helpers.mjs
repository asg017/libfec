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
