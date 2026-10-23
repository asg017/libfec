// Every filing in a directory, header and cover only (no itemizations are read).
import { readdirSync } from "node:fs";
import { join } from "node:path";
import { Filing } from "../dist/index.js";
import { FIXTURES, arg } from "./_fixture.mjs";

const dir = arg(FIXTURES);
for (const name of readdirSync(dir).filter((f) => f.endsWith(".fec")).sort()) {
  const filing = new Filing(join(dir, name));
  filing.close();
  console.log(`${name}\t${filing.fecVersion}\t${filing.formType}\t${filing.cover.type}\t${filing.filerName}`);
}
