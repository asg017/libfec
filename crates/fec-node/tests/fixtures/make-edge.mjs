// Writes edge.fec: a v8.5 F3N filing whose rows each exercise one edge case
// (see tests/edge.test.mjs, which asserts every row), and unknown-cover.fec
// (a cover with no typed struct). Rerun after changing it, commit the
// outputs and regenerate the parity fixtures (make-typed-json.sh):
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

// [description, fields] in file order; line = index + 3 (but the last row:
// the quoted \n in "quote with separators" adds a line).
//
// The rows with no column mapping come last: a JS pass ends at the first
// one it throws for (Python's carries on), so everything before them stays
// reachable from both views.
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
  ["SA3L", row("SA3L", { filer_committee_id_number: "C00900860", transaction_id: "L1", lobbyist_registrant_last_name: "Lobby", bundled_amount_period: "10" })],
  ["SC/10", row("SC/10", { filer_committee_id_number: "C00900860", transaction_id_number: "C1", lender_organization_name: "Bank", loan_amount_original: "100" })],
  ["SC1/10", row("SC1/10", { filer_committee_id_number: "C00900860", transaction_id_number: "C2", lender_organization_name: "Bank", loan_amount: "100" })],
  ["mapped, no struct", row("F3S", { filer_committee_id_number: "C00900860" })],
  ["summary record, no struct", row("F3PS", { filer_committee_id_number: "C00900860", net_contributions: "125.50", a_i_individuals_itemized: "7" })],
  // Quotes last: quoting is off (only a pair wrapping a whole field comes
  // off), so a quoted \n still ends the line and a quoted \x1c still splits
  // fields. The quoted \n splits off a line `C"…`, which has no mapping.
  ["quoted name", sa({ transaction_id: "E9", contributor_last_name: '"O""Brien"' })],
  ["final normal row", sa({ transaction_id: "E11", contributor_last_name: "Last" })],
  ["quote with separators", sa({ transaction_id: "E10", contributor_last_name: '"A\x1cB\nC"' })],
  ["unmapped", ["ZZ9", "C00900860", "x"]],
];

const text = [hdr, cover, ...ROWS.map(([, fields]) => fields.join("\x1c"))].join("\n") + "\n";
// latin1: every char is one byte, so "\x92" is the raw cp1252 byte; encode
// the real Unicode rows as UTF-8 first.
const bytes = Buffer.concat(
  text.split(/(\x92)/).map((part) => (part === "\x92" ? Buffer.from([0x92]) : Buffer.from(part, "utf8"))),
);
writeFileSync(new URL("edge.fec", import.meta.url), bytes);
console.log(`edge.fec: ${ROWS.length} rows`);

// unknown-cover.fec: a cover with no typed struct (an F3PS line where the
// cover goes), so `filing.cover` is an Unknown, then two ordinary rows.
const unknownCover = [
  hdr,
  row("F3PS", { filer_committee_id_number: "C00900860", net_contributions: "10.00" }).join("\x1c"),
  sa({ transaction_id: "U1" }).join("\x1c"),
  row("TEXT", { filer_committee_id_number: "C00900860", transaction_id_number: "U2", text: "Cover note" }).join("\x1c"),
].join("\n") + "\n";
writeFileSync(new URL("unknown-cover.fec", import.meta.url), unknownCover, "utf8");
console.log("unknown-cover.fec: F3PS cover, 2 rows");
