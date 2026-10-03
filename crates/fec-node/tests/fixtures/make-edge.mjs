// Writes edge.fec: a v8.5 F3N filing whose rows each exercise one edge case
// (see tests/edge.test.mjs, which asserts every row). Rerun after changing
// it, and commit both files:
//
//   node tests/fixtures/make-edge.mjs   (needs `make build` for columns())
import { readFileSync, writeFileSync } from "node:fs";
import { columns } from "../../dist/index.js";

const SAMPLE = new URL("../../../fec-py/tests/fixtures/1921705.fec", import.meta.url);
const [hdr, cover] = readFileSync(SAMPLE, "latin1").split("\n");

/** A row of `rowType` with every mapped column blank except `values`. */
function row(rowType, values = {}) {
  const cols = columns(rowType, "8.5").map((c) => c.name);
  const fields = cols.map((c) => values[c] ?? "");
  fields[0] = rowType;
  for (const k of Object.keys(values)) {
    if (!cols.includes(k)) throw new Error(`${rowType} has no column ${k}`);
  }
  return fields;
}

const sa = (values) =>
  row("SA11AI", {
    filer_committee_id_number: "C00900860",
    entity_type: "IND",
    contributor_last_name: "Smith",
    contributor_first_name: "Ann",
    contribution_date: "20250707",
    contribution_amount: "50.00",
    ...values,
  });

// [description, fields] in file order; line = index + 3.
const ROWS = [
  ["BMP non-ASCII", sa({ transaction_id: "E1", contributor_last_name: "Muñoz" })],
  ["astral", sa({ transaction_id: "E2", contributor_last_name: "😀Smith" })],
  ["lossy cp1252 byte", sa({ transaction_id: "E3", contributor_last_name: "O\x92Neil" })],
  ["garbage date", sa({ transaction_id: "E4", contribution_date: "20230230" })],
  ["garbage amount", sa({ transaction_id: "E5", contribution_amount: "1,000.00" })],
  ["blank amount", sa({ transaction_id: "E6", contribution_amount: "  " })],
  ["short row", sa({ transaction_id: "E7" }).slice(0, 10)],
  ["extra fields", [...sa({ transaction_id: "E8" }), "x", "y", "z"]],
  ["TEXT", row("TEXT", { filer_committee_id_number: "C00900860", transaction_id_number: "T1", text: "A note" })],
  ["unmapped", ["ZZ9", "C00900860", "x"]],
  ["SA3L", row("SA3L", { filer_committee_id_number: "C00900860", transaction_id: "L1", lobbyist_registrant_last_name: "Lobby", bundled_amount_period: "10" })],
  ["SC/10", row("SC/10", { filer_committee_id_number: "C00900860", transaction_id_number: "C1", lender_organization_name: "Bank", loan_amount_original: "100" })],
  ["SC1/10", row("SC1/10", { filer_committee_id_number: "C00900860", transaction_id_number: "C2", lender_organization_name: "Bank", loan_amount: "100" })],
  ["mapped, no struct", row("F3S", { filer_committee_id_number: "C00900860" })],
  // Quotes last: the parser reads quotes literally (quoting is off), so a
  // quoted \n still ends the line and a quoted \x1c still splits fields.
  ["quoted name", sa({ transaction_id: "E9", contributor_last_name: '"O""Brien"' })],
  ["quote with separators", sa({ transaction_id: "E10", contributor_last_name: '"A\x1cB\nC"' })],
  ["final normal row", sa({ transaction_id: "E11", contributor_last_name: "Last" })],
];

const text = [hdr, cover, ...ROWS.map(([, fields]) => fields.join("\x1c"))].join("\n") + "\n";
// latin1: every char is one byte, so "\x92" is the raw cp1252 byte; encode
// the real Unicode rows as UTF-8 first.
const bytes = Buffer.concat(
  text.split(/(\x92)/).map((part) => (part === "\x92" ? Buffer.from([0x92]) : Buffer.from(part, "utf8"))),
);
writeFileSync(new URL("edge.fec", import.meta.url), bytes);
console.log(`edge.fec: ${ROWS.length} rows`);
