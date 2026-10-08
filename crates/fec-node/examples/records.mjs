// The raw path: records() with columns() for the names.
import { columns, open } from "../dist/index.js";
import { PAC, arg } from "./_fixture.mjs";

const filing = open(arg(PAC));
try {
  for (const { rowType, line, fields } of filing.records()) {
    const names = columns(rowType, filing.fecVersion).map((c) => c.name);
    const at = names.indexOf("contribution_amount");
    console.log(line, rowType, names[7], fields[7], "contribution_amount", fields[at]);
    break;
  }
} finally {
  filing.close();
}
