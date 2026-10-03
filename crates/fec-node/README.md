# @asg017/libfec

Parse FEC electronic filings (`.fec`) in Node, Deno and Bun with
[libfec](https://github.com/asg017/libfec)'s Rust parser (via [napi-rs](https://napi.rs)).
Typed covers and itemizations with TypeScript types for every form and schedule.

> **Alpha.** Not on npm. Tarballs are attached to
> [GitHub Releases](https://github.com/asg017/libfec/releases). The API may still change.

```js
import { open } from "@asg017/libfec";

const filing = open("1721696.fec");
try {
  for (const row of filing.itemizations()) {
    switch (row.itemization.type) {
      case "ScheduleA":
        console.log(row.itemization.contributor.name.last_name, row.itemization.contribution_amount);
        break; // "Aaronson" 104.17, …
      case "ScheduleB":
        console.log(row.itemization.payee.organization_name, row.itemization.expenditure_amount);
        break;
    }
  }
} finally {
  filing.close();
}
```

On Node 24+, Deno and Bun, `using filing = open("1721696.fec")` closes it for you. Node 22
doesn't parse `using`, so the examples here use `try`/`finally`.

The field-by-field reference, generated from the Rust structs' docs, is the
[Node.js API reference](https://asg017.github.io/libfec/reference/node/).

## Install

Supported: Node 22+, Deno 2+, Bun 1.2+ on macOS (arm64, x64), Linux glibc and musl (x64,
arm64) and Windows x64. Anywhere else, build from source (a Rust toolchain, then
`npm run build` in `crates/fec-node`).

```bash
npm i https://github.com/asg017/libfec/releases/download/v$VERSION/asg017-libfec-$VERSION.tgz
bun add https://github.com/asg017/libfec/releases/download/v$VERSION/asg017-libfec-$VERSION.tgz
```

**Deno** can't install a tarball URL (`deno install` fails with "Not implemented scheme
'https'"). Download and extract it, then import the extracted package:

```bash
mkdir -p vendor/libfec
curl -L https://github.com/asg017/libfec/releases/download/v$VERSION/asg017-libfec-$VERSION.tgz \
  | tar xz -C vendor/libfec --strip-components 1
```

```js
import { open } from "./vendor/libfec/dist/index.js";
```

Run with `deno run --allow-read --allow-ffi --allow-env`: `--allow-read` to read filings and
the native addon, `--allow-ffi` to load the addon, and `--allow-env` because napi-rs's loader
reads `NAPI_RS_*` environment variables.

## Reading a filing

`open(source, options?)` returns a `FilingReader`. It reads the header and cover right away,
then rows as you iterate. `source` is a path, a `file:` URL, or the file's bytes
(`Uint8Array`/`Buffer`/`ArrayBuffer`). Bytes are read in place, without copying, so don't
modify them while the reader is open. `reader.id` is the filing ID from the file name
(`"1721696"`), or the `id` option.

Read each reader **once**, with one of:

| Method | Yields | 91 MB filing |
|---|---|---|
| `itemizations()` | `Row<Itemization>`: typed rows only (rows with no typed struct are skipped) | ~1.3 s |
| `rows()` / `for…of` | `Row<null>`: every row after the cover, untyped | ~0.4 s |
| `records()` | `{ rowType, line, fields }`: raw strings, no column mapping needed | ~0.4 s |

A second call on the same reader throws `FecError` with `code: "ERR_FILING_CONSUMED"`; open
the file again to read it again. None of them takes a filter: `switch` on
`row.itemization.type`, or test `row.rowType`. (Python's `rows("SA")` prefix filter has no JS
equivalent.) They return generators, so iterator helpers work:

```js
const firstThree = filing
  .rows()
  .filter((row) => row.rowType === "SA11AI")
  .take(3)
  .map((row) => [row.values.contributor_last_name, row.values.contribution_amount])
  .toArray(); // [["Aaronson", 104.17], ["Aaronson", 104.17], ["Aarts", 20.84]]
```

(In TypeScript, add `"ESNext.Iterator"` to `lib` for the helpers' types.)

`read(source, options?)` reads every row into memory instead and returns a `Filing`:
`filing.rows` holds the untyped rows, and `filing.itemizations()` returns the typed ones.
That re-reads the source the first time and caches the result. A `Filing` can be iterated
any number of times and has nothing to close.

`readHeader(source)` reads only the header.

### Options

| Option | Default | |
|---|---|---|
| `dates` | `"iso"` | `"date"` turns date columns of `row.values` (and `coverSummary`'s dates) into `Date`s at UTC midnight. Typed records always use ISO strings. |
| `unknownRows` | `"throw"` | What `rows()` does with a row type that has no column mapping: throw `MissingMappingError`, or `"skip"` it and list it in `skippedRows` |
| `batchSize` | `1024` | Rows per native batch: a tuning knob, results don't change |
| `id` | | The filing ID for `reader.id`/`filing.id`. Bytes have no file name, so without it their `id` is `null`: `open(bytes, { id: "1721696" })`. For a path it replaces the ID taken from the file name. |

## Typed records

`row.itemization` is an `Itemization`, a union discriminated on **`type`**, the name of the
Rust struct it comes from. A `switch` narrows it, the TypeScript counterpart of Python's
`match`/`case`. A `never` default makes the compiler flag any type you didn't handle:

```ts
import type { Itemization } from "@asg017/libfec";

function amount(it: Itemization): number | null {
  switch (it.type) {
    case "ScheduleA": return it.contribution_amount;
    case "ScheduleB": return it.expenditure_amount;
    default: return null; // or `const _: never = it;` once every type is listed
  }
}
```

Every `Itemization` `type` (also exported as `ITEMIZATION_TYPES`):
`ScheduleA`, `ScheduleB`, `ScheduleD`, `ScheduleF`, `ScheduleH1`, `ScheduleH2`, `ScheduleH3`,
`ScheduleH4`, `ScheduleH5`, `ScheduleH6`, `Form5Contribution`, `Form5Expenditure`,
`Form6Contribution`, `Form7Communication`, `Form9ControllingPerson`, `Form9Donation`,
`Form9Disbursement`, `Form9Candidate`, `Form13Donation`, `Form13Refund`, `ScheduleL`,
`TextRecord`, `ScheduleA3L`, `ScheduleE`, `ScheduleC`, `ScheduleC1`, `ScheduleC2`.

There is one struct per schedule across every FEC version. Field names are the v8.x meanings
(`transaction_id` everywhere), keys are snake_case, and people, organizations, addresses and
candidates are nested objects (`Entity`, `PersonName`, `Address`, `CandidateRef`).

The cover works the same way. `filing.cover` is a `Cover` (or `null` for a form with no typed
struct), tagged with `type`:

```js
switch (filing.cover?.type) {
  case "Form3X":
    console.log(filing.cover.committee_name, filing.cover.summary.line6c_total_receipts.column_a);
    break; // "PFIZER INC. PAC" 83741.93
  case "Form99":
    console.log(filing.cover.text);
    break;
}
```

Every `Cover` `type` (also `COVER_TYPES`): `Form1`, `Form3`, `Form3P`, `Form1M`, `Form3X`,
`Form3L`, `Form4`, `Form7`, `Form13`, `Form24`, `Form5`, `Form6`, `Form9`, `Form2`, `Form99`.

In Rust and in `libfec`'s JSON output, itemizations carry a `family` tag (`"SA"`) and covers
are `{ form, data }`. The JS layer replaces both with `type`.

Typed values follow the parser:

| Rust | TS | Blank | Unparseable |
|---|---|---|---|
| `f64` (the transaction amount) | `number` | `0` | **`0`** |
| `Option<f64>` (aggregates, …) | `number \| null` | `null` | `null` |
| `Option<Date>` | `string \| null` (`"YYYY-MM-DD"`) | `null` | `null` |
| `Option<String>` | `string \| null` | `null` | |
| `bool` (`memo`, …) | `boolean` | `false` | |

So an unparseable amount reads `0`, the same as in Python's typed classes. The same row's
`values` keeps the raw text, and the reader lists it in `invalidValues`.

Label functions turn codes into names: `entityTypeLabel("IND")` → `"Individual"`,
`electionCodeLabel`, `officeLabel`, `partyLabel`, `supportOpposeLabel`, `categoryCodeLabel`.

## Rows

A `Row` mirrors Python's `libfec_parser.Row`:

| Python `Row` | JS `Row` |
|---|---|
| `row.row_type` | `row.rowType` |
| `row.line` | `row.line` (1-based physical line) |
| `row.itemization` | `row.itemization` (from `itemizations()`; `null` from `rows()`) |
| `row.extra_fields` | `row.extraFields` |
| `row["contribution_amount"]`, `keys()`/`items()` | `row.values.contribution_amount`, `Object.keys(row.values)`, … |
| `row[20]`, `row.fields()` | `row.fields[20]`, `row.fields` |

`row.values` is a plain object keyed by FEC column name, in column order.
`JSON.stringify(row)` gives `{ rowType, line, values, itemization }`.

`TEXT` rows' first column is `rec_type`, not `form_type`, so test `row.rowType`, which works
for every row. Typed, they are `type: "TextRecord"`.

### Values

| Column kind | Value in `row.values` |
|---|---|
| text | the raw string, untrimmed (`""` stays `""`) |
| amount | a `number`; `null` if blank; **the raw string** if it doesn't parse (Rust's `f64` grammar, so `"1,000.00"` and `"0x10"` don't) |
| date | `"YYYY-MM-DD"` (or a `Date`, see `dates`); `null` if blank; **`null`** if it isn't a real `YYYYMMDD` date |
| any, past the end of a short row | `null` |

**Unlike the Python package**, an invalid date is `null` here, not the raw string. That way
every date is either a valid ISO date or `null`. The raw text is in `row.fields` and
`records()`.

Unparseable amounts and invalid dates are listed in `reader.invalidValues` (`{ line, column,
raw }`, the first 1,000), with `reader.invalidValueCount` counting all of them. Nothing is
logged.

`columns(rowType, fecVersion)` lists a row type's columns and kinds:
`columns("SA11AI", "8.4")[20]` is `{ name: "contribution_amount", kind: "amount" }`.

## The cover and header

`header`, `cover`, `coverSummary` and `coverRow` are read when the filing is opened, and stay
readable after `close()`:

- `filing.header`: the `HDR` record (`fecVersion`, `softwareName`, …).
- `filing.cover`: the typed cover (above).
- `filing.coverSummary`: the six fields every form has, as in Python's `Cover`: `formType`,
  `filerId`, `filerName`, `reportCode`, `coverageFromDate`, `coverageThroughDate`.
  For `1721696.fec`, that's `"F3XN"`, `"C00016683"`, `"PFIZER INC. PAC"`, `"M8"`,
  `"2023-07-01"`, `"2023-07-31"`.
- `filing.coverRow`: the cover line as a `Row` (123 columns for an F3XN).

## Errors

| Error | When |
|---|---|
| Node-style `Error` with `code: "ENOENT"` (`EACCES`, …), `path`, `syscall` | the path can't be opened, exactly as `fs.openSync` reports it |
| `FecParseError` (`code: "FEC_PARSE"`) | the header, cover or a row can't be parsed |
| `MissingMappingError` (`code: "FEC_MISSING_MAPPING"`; `rowType`, `fecVersion`, `line`) | `rows()` meets an unmapped row type (with `unknownRows: "throw"`), or `columns()` |
| `FecError` with `code: "ERR_FILING_CONSUMED"` | a second `itemizations()`/`rows()`/`records()`/`for…of` on one reader |
| `FecError` with `code: "ERR_FILING_CLOSED"` | reading after `close()` |
| `TypeError` | a bad source or option, or a filter argument |

`FecParseError` and `MissingMappingError` extend `FecError`, which extends `Error`.

## Performance

`benchmarks/node/bench.mjs` on a 91 MB filing (408,160 rows), on macOS arm64:

| | Node 24 | Deno 2.9 | Bun 1.2 |
|---|---|---|---|
| `itemizations()` | 1.31 s | 1.28 s | 1.33 s |
| `rows()` | 0.39 s | 0.37 s | 0.41 s |
| `records()` | 0.41 s | 0.38 s | 0.32 s |
| `read()` | 0.56 s | 0.56 s | 0.56 s |

Typing every row costs about three times the raw path, and most of that is the Rust
parser's typing (~0.6 s), so use `rows()` or `records()` when you don't need typed records.

## Known issues

These come from the parser and are the same in the Python package:

- An F99's `[BEGINTEXT]` body is on `filing.cover.text`, but `[BEGINTEXT]` blocks elsewhere
  are skipped.
- A few column names are placeholders ending in `_TODO_DUP`.
- Text that isn't valid UTF-8 is decoded lossily (an invalid byte becomes `U+FFFD`). For
  example, a cp1252 `’` becomes `�`.
- Quotes are literal: the parser doesn't unquote CSV-style quoted fields, so a quoted field
  holding a separator or a newline still splits.

## Examples

`examples/*.mjs` run under all three runtimes (`make examples`, and in CI). Each takes an
optional filing path and defaults to a committed fixture.

## Development

```bash
make install      # npm ci
make build        # debug addon (native/) + TypeScript (dist/)
make test         # the node:test suite under node, deno and bun
make typecheck    # tsc over js/, the compile-only type tests and the examples
make gen-types    # regenerate js/generated/ from fec-parser (commit the result)
make docs         # TypeDoc -> site/reference/node
make bench        # benchmarks/node/bench.mjs (release build)
make test-slow    # perf guards on benchmarks/1805248.fec
```
