// Every filing in a directory, header and cover only (no rows are read).
import { readdirSync } from "node:fs";
import { join } from "node:path";
import { open } from "../dist/index.js";
import { FIXTURES, arg } from "./_fixture.mjs";

const dir = arg(FIXTURES);
for (const name of readdirSync(dir).filter((f) => f.endsWith(".fec")).sort()) {
  const filing = open(join(dir, name));
  filing.close();
  const { formType, filerName } = filing.coverSummary;
  console.log(`${name}\t${filing.fecVersion}\t${formType}\t${filing.cover?.type ?? "-"}\t${filerName}`);
}
