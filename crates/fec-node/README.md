# @alex.garcia/libfec

Parse FEC electronic filings (`.fec`) in Node, Deno and Bun with
[libfec](https://github.com/asg017/libfec)'s Rust parser (via [napi-rs](https://napi.rs)).
Typed covers and itemizations with TypeScript types for every form and schedule. The API
mirrors the Python package's [`libfec.Filing`](../fec-py/README.md#native-api).

> **Alpha.** Not on npm. Tarballs are attached to
> [GitHub Releases](https://github.com/asg017/libfec/releases). The API may still change.

```js
import { Filing } from "@alex.garcia/libfec";

const filing = new Filing("1721696.fec");
try {
  console.log(filing.filerName, filing.cover.type); // "PFIZER INC. PAC" "Form3X"
  for (const item of filing.itemizations()) {
    switch (item.type) {
      case "ScheduleA":
        console.log(item.contributor.name.last_name, item.contribution_amount);
        break; // "Aaronson" 104.17, …
      case "ScheduleB":
        console.log(item.payee.organization_name, item.expenditure_amount);
        break;
    }
  }
} finally {
  filing.close();
}
```

On Node 24+, Deno and Bun, `using filing = new Filing("1721696.fec")` closes it for you.
Node 22 doesn't parse `using`, so the examples here use `try`/`finally`.

The field-by-field reference, generated from the Rust structs' docs, is the
[Node.js API reference](https://asg017.github.io/libfec/reference/node/).

## Breaking changes: the `Filing` API

The API was reshaped to match the Python package. There are no aliases for the old names:

| Before | Now |
|---|---|
| `open(src)` → `FilingReader`; `read(src)` → eager `Filing` | `new Filing(src, options?)` |
| — | `await Filing.fromId(id, options?)` |
| `filing.cover: Cover \| null` / `filing.coverRow` | `filing.cover: Cover \| Unknown` / `filing.coverRecord` |
| `filing.coverSummary.formType`, `.filerId`, … | `filing.formType`, `filing.filerId`, … |
| `itemizations()`: skips untyped rows, yields `Row<Itemization>` | `itemizations()`: every record, as an `Itemization` or an `Unknown` |
| `rows()` → `Row<null>` | `itemizationRecords()` → `Record` |
| `Row` | `Record` |
| — | `Unknown` (`type: "Unknown"`, `formType`, `record`, `toJSON()`) |
| — | `filing.sourceLength` |
| `records()`, `[Symbol.iterator]`, `ERR_FILING_CONSUMED` | removed: every call is a new pass |
| `invalidValues`, `invalidValueCount`, `skippedRows`, the `unknownRows` option | removed |

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
import { Filing } from "./vendor/libfec/dist/index.js";
```

Run with `deno run --allow-read --allow-ffi --allow-env`: `--allow-read` to read filings and
the native addon, `--allow-ffi` to load the addon, and `--allow-env` because napi-rs's loader
reads `NAPI_RS_*` environment variables. `Filing.fromId` and `rss.fetch` also need
`--allow-net`.

### In the browser

Bundlers resolve `@alex.garcia/libfec` to a WebAssembly build of the same API through the
`"browser"` export condition, and `@alex.garcia/libfec/browser` imports it explicitly. It's
also published on its own as `@alex.garcia/libfec-wasm`. It reads bytes instead of paths
(`new Filing(new Uint8Array(await file.arrayBuffer()))`), and `Filing.fromId` and
`rss.fetch` need a `proxy` because the FEC sends no CORS headers. Everything else behaves
the same, and the test suite checks that on every fixture. See
[`crates/fec-wasm`](../fec-wasm/README.md).

## `Filing`

```js
import { Filing } from "@alex.garcia/libfec";
```

`new Filing(source, options?)` opens a filing from a path, a `file:` URL or bytes (see
[Sources](#sources)), and `await Filing.fromId(1721696)` downloads one from the FEC (see
[`fromId`](#fromid)). The `HDR` and cover records are parsed in the constructor, so `header`,
`cover` and the filer attributes are there right away. Everything after the cover is
streamed, a batch at a time, by one of two views of the same records:

| | Typed | Raw |
|---|---|---|
| Cover | `filing.cover`: `Form3X`, `Form1`, … | `filing.coverRecord`: a `Record` |
| Everything after it | `filing.itemizations()`: `ScheduleA`, `ScheduleB`, … | `filing.itemizationRecords()`: `Record`s |

Anything without a typed class (a summary record such as `F3PS`, a cover form with no
struct) comes back as an [`Unknown`](#unknown) rather than `null`, so the typed views are
total and line up one-to-one with the raw ones.

```js
const filing = new Filing("1721696.fec");
filing.itemizations().next().value;        // { type: "ScheduleA", form_type: "SA11AI", … }
filing.itemizationRecords().next().value;  // Record { rowType: "SA11AI", line: 3, … }
```

### Sources

`new Filing()` and `readHeader()` accept the same kinds of source:

- a filesystem path, as a `string`, or a `file:` `URL`. **A string is always a path**, never
  the filing's contents: pass bytes if you already have the data in memory.
- bytes: a `Uint8Array` (a `Buffer` is one) or an `ArrayBuffer`. They are read in place,
  without copying, so don't modify them while the filing is open.

A path that can't be opened throws the same error `fs.openSync` would (`code: "ENOENT"`,
`path`, `syscall: "open"`), and anything else throws a `TypeError`.

### Attributes

```js
const filing = new Filing("1721696.fec");
filing.id                    // "1721696": from the file name, or the `id` option
filing.fecVersion            // "8.4": shortcut for filing.header.fecVersion
filing.formType              // "F3XN"
filing.filerId               // "C00016683"
filing.filerName             // "PFIZER INC. PAC"
filing.reportCode            // "M8"
filing.coverageFromDate      // "2023-07-01" (a Date with `dates: "date"`)
filing.coverageThroughDate   // "2023-07-31"
filing.sourceLength          // 263105: the source's size in bytes
```

The six filer attributes are normalized across every form type, whatever the form itself
calls them (`committee_name`, `filer_name`, `organization_name`, …). `reportCode` and the
coverage dates are `null` for forms that are not periodic reports (F1, F2, F24, F99, …).
`id` is `null` for bytes without the `id` option.

### Passes

- **Each call to `itemizations()` or `itemizationRecords()` is a new pass** from the top of
  the filing, so you can read it as many times as you like, one pass after another or
  interleaved. A single pass is an ordinary generator: once exhausted, it stays exhausted.
- A pass opens its native reader on its first `next()`, not when you call the method, so a
  pass you never step costs nothing. It closes the reader when it is exhausted,
  `return()`ed (a `break` out of `for…of`) or throws; one you start and drop midway is closed
  when it's garbage-collected, or by `close()`, whichever comes first.
- `close()` releases the source and stops every live pass: a running pass throws
  `FecError` with `code: "ERR_FILING_CLOSED"` on its next step, and so does a new call. It's
  idempotent, and `header`, `cover`, `coverRecord` and the filer attributes stay readable
  after it. `filing[Symbol.dispose]()` is `close()`, for `using`.
- Neither method takes a filter: `switch` on `item.type`, or test `record.rowType`.
  (Python's prefix filters have no JS equivalent.) They return generators, so iterator
  helpers work:

```js
const firstThree = filing
  .itemizationRecords()
  .filter((record) => record.rowType === "SA11AI")
  .take(3)
  .map((record) => [record.values.contributor_last_name, record.values.contribution_amount])
  .toArray(); // [["Aaronson", 104.17], ["Aaronson", 104.17], ["Aarts", 20.84]]
```

(In TypeScript, add `"ESNext.Iterator"` to `lib` for the helpers' types.)

### Options

| Option | Default | |
|---|---|---|
| `dates` | `"iso"` | `"date"` turns date columns of `record.values` (and the coverage dates) into `Date`s at UTC midnight. Typed records always use ISO strings. |
| `id` | | The filing ID for `filing.id`. Bytes have no file name, so without it their `id` is `null`: `new Filing(bytes, { id: "1721696" })`. For a path it replaces the ID taken from the file name. |
| `batchSize` | `1024` | Rows per native batch: a tuning knob, results don't change |

An unknown option, or a bad value for one, throws a `TypeError`.

## `fromId`

`Filing.fromId()` downloads a filing from `docquery.fec.gov` by its ID, as a number or a
string (`1721696`, `"1721696"` and `"FEC-1721696"` all work), with the runtime's global
`fetch()`:

```js
const filing = await Filing.fromId(1721696);   // Filing.fromId(1721696, { timeout: 60_000 })
filing.id          // "1721696"
filing.filerName   // "PFIZER INC. PAC"
```

The whole file is read into memory, then opened as bytes. It takes the `dates` and
`batchSize` options, plus:

| Option | Default | |
|---|---|---|
| `proxy` | | Fetch through a proxy: a URL prefix (`"https://proxy.example/?url="`), a template containing `{url}`, or a function from the filing URL to the URL to fetch |
| `signal` | | An `AbortSignal` to cancel the download |
| `timeout` | `60000` | Milliseconds before the download is aborted (`Infinity` for none) |

- A malformed ID or option throws a `TypeError` before any request.
- No such filing (HTTP 404) throws `FecError` with `code: "ERR_FILING_NOT_FOUND"`. Any other
  non-2xx response throws `FecError` with `code: "ERR_HTTP"`. Both set `status`.
- A network failure, an abort or the timeout throws whatever `fetch()` threw.
- Like Python's `Filing.from_id`, there is no fallback to paper filings.
- **In a browser, pass a `proxy`.** docquery sends no CORS headers, so the browser blocks the
  response. Without a proxy that fails with a `TypeError` with `code: "ERR_CORS"` that says
  so (the original error is its `cause`). Node, Deno and Bun don't need one.

## Typed covers

`filing.cover` is the cover as a typed object, a `Cover` union discriminated on **`type`**,
the name of the Rust struct it comes from, or an [`Unknown`](#unknown) for a form with no
struct. A `switch` narrows it, the TypeScript counterpart of Python's `match`/`case`:

```js
switch (filing.cover.type) {
  case "Form3X":
    console.log(filing.cover.committee_name, filing.cover.summary.line6c_total_receipts.column_a);
    break; // "PFIZER INC. PAC" 83741.93
  case "Form99":
    console.log(filing.cover.text);
    break;
  case "Unknown":
    console.log(filing.cover.formType, filing.cover.record.values);
    break;
}
```

Every `Cover` `type` (also `COVER_TYPES`): `Form1`, `Form3`, `Form3P`, `Form1M`, `Form3X`,
`Form3L`, `Form4`, `Form7`, `Form13`, `Form24`, `Form5`, `Form6`, `Form9`, `Form2`, `Form99`.

The fields are named for what they mean on the paper form and grouped the way the form
groups them. They are the same as `libfec info -f json`'s `cover_data` and Python's cover
classes.

## Typed itemizations

`filing.itemizations()` yields one typed object per record, in file order: an
`Itemization`, discriminated on `type` like the cover, or an [`Unknown`](#unknown). A
`never` default makes the compiler flag any type you didn't handle:

```ts
import type { Itemization, Unknown } from "@alex.garcia/libfec";

function amount(item: Itemization | Unknown): number | null {
  switch (item.type) {
    case "ScheduleA": return item.contribution_amount;
    case "ScheduleB": return item.expenditure_amount;
    default: return null; // or `const _: never = item;` once every type is listed
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
`item.form_type` is the row type as filed (`"SA11AI"`). Typed objects are plain data, so
`JSON.stringify` and `postMessage` work on them.

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

So an unparseable amount reads `0`, the same as in Python's typed classes. The same
record's `values` keeps the raw text.

Label functions turn codes into names: `entityTypeLabel("IND")` → `"Individual"`,
`electionCodeLabel`, `officeLabel`, `partyLabel`, `supportOpposeLabel`, `categoryCodeLabel`.

Typing every record costs more than reading it raw (see [Performance](#performance)). When
you only need a few columns, or the exact raw text, use `itemizationRecords()`;
`record.itemization` types a single `Record` on demand.

## `Unknown`

What the typed views yield for a record with no typed class:

```js
u.type       // "Unknown": the discriminant, so `switch (item.type)` narrows it
u.formType   // "F1S": the row type as filed
u.record     // the underlying Record
u.toJSON()   // { type: "Unknown", formType, values }: its values by column name
```

## `Header`

`filing.header` is the `HDR` record, and `readHeader(source)` reads only that, from any
source `new Filing()` accepts: `recordType`, `efType`, `fecVersion`, `softwareName`,
`softwareVersion`, `reportId`, `reportNumber`, `comment`, plus the legacy and paper header
fields (`style`, `isPaper`, `batchNumber`, `receivedDate`, …).

### Streaming: `coverEnd()`

To stop downloading a filing once its header and cover are in, call `coverEnd(prefix)` on
what you have so far. It returns how many bytes settle the header and cover, or `null`
while bytes after `prefix` could still change them (an F99's `[BEGINTEXT]` block counts as
part of its cover). Bytes after that length can't change `header`, `cover` or
`coverRecord`, or the error opening the filing raises.

```js
let buf = new Uint8Array(0);
for await (const chunk of response.body) {
  buf = concat(buf, chunk);
  const n = coverEnd(buf);
  if (n !== null) return new Filing(buf.subarray(0, n)); // and cancel the download
}
return new Filing(buf); // the whole file
```

The `Filing` is an ordinary one over those `n` bytes: its `sourceLength` is `n`, and its
itemizations are only the complete rows before `n`, if there are any.

## `Record`

A `Record` is one raw line of the filing: its values by column name, plus the raw fields by
position. `itemizationRecords()` yields them, and `coverRecord` is the cover line as one,
with every column the form defines (123 for an F3XN 8.4 cover). It mirrors Python's
`libfec.Record`:

| Python `Record` | JS `Record` |
|---|---|
| `record.row_type` | `record.rowType` |
| `record.line` | `record.line` (1-based physical line) |
| `record.itemization` | `record.itemization` (typed anew on each access; `null` if it isn't one) |
| `record.extra_fields` | `record.extraFields` |
| `record["contribution_amount"]`, `keys()`/`items()` | `record.values.contribution_amount` or `record.get("contribution_amount")`, `Object.keys(record.values)`, … |
| `record[20]`, `record.fields()` | `record.fields[20]`, `record.fields` |

```js
const record = filing.itemizationRecords().next().value;
record.values.contribution_amount     // 104.17 (number)
record.fields[20]                     // "104.17" (string): the same field, by position
record.values.contributor_last_name   // "Aaronson"
```

`record.values` is a plain object keyed by FEC column name, in column order. Its data are
own properties, so `{ ...record }` and `structuredClone(record)` give plain objects.
`JSON.stringify(record)` gives `{ rowType, line, values }`.

`TEXT` records' first column is `rec_type`, not `form_type`, so test `record.rowType`, which
works for every record. Typed, they are `type: "TextRecord"`.

**In TypeScript, importing `Record` shadows the global `Record<K, V>` utility type** in that
module. Write `{ [key: string]: V }` there, or import it under another name
(`import { Record as FecRecord } from "@alex.garcia/libfec"`).

### Values

| Column kind | Value in `record.values` |
|---|---|
| text | the raw string, untrimmed (`""` stays `""`) |
| amount | a `number`; `null` if blank; **the raw string** if it doesn't parse (Rust's `f64` grammar, so `"1,000.00"` and `"0x10"` don't) |
| date | `"YYYY-MM-DD"` (or a `Date`, see `dates`); `null` if blank; **`null`** if it isn't a real `YYYYMMDD` date |
| any, past the end of a short record | `null` |

Because an unparseable amount comes back as a string, guard with `typeof v === "number"`
rather than assuming every amount parsed. **Unlike the Python package**, an invalid date is
`null` here, not the raw string. That way every date is either a valid ISO date or `null`.
The raw text is in `record.fields`.

`columns(rowType, fecVersion)` lists a row type's columns and kinds:
`columns("SA11AI", "8.4")[20]` is `{ name: "contribution_amount", kind: "amount" }`.

## Errors

| Error | When |
|---|---|
| Node-style `Error` with `code: "ENOENT"` (`EACCES`, …), `path`, `syscall` | the path can't be opened, exactly as `fs.openSync` reports it |
| `FecParseError` (`code: "FEC_PARSE"`) | the header, cover or a record can't be parsed |
| `MissingMappingError` (`code: "FEC_MISSING_MAPPING"`; `rowType`, `fecVersion`, `line`) | a record's row type has no column mapping in the filing's FEC version (see below), or `columns()` |
| `FecError` with `code: "ERR_FILING_CLOSED"` | `itemizations()`/`itemizationRecords()` after `close()`, or the next step of a pass that was running |
| `FecError` with `code: "ERR_FILING_NOT_FOUND"` (`status: 404`) | `Filing.fromId`: docquery has no such filing |
| `FecError` with `code: "ERR_HTTP"` (`status`) | `Filing.fromId`: any other non-2xx response |
| `TypeError` | a bad source or option, an argument to `itemizations()`/`itemizationRecords()`, a malformed filing ID; in a browser, `code: "ERR_CORS"` for a blocked `fromId` or `rss.fetch` |
| `RangeError` | an unknown `rss` preset |

`FecParseError` and `MissingMappingError` extend `FecError`, which extends `Error`. Network
failures, aborts and timeouts are the runtime's own `fetch()` errors.

**Missing mappings end the pass.** A record whose row type has no column mapping throws
`MissingMappingError` from the `next()` that reaches it, in `itemizationRecords()` and, for a
record with no typed class, in `itemizations()`. The records before it are still yielded. A
JS generator can't resume after it throws, so that pass is over; start a new one to read the
filing again. (Python's pass stays usable after the raise.)

## The RSS feed

`rss` reads the FEC's e-filing RSS feed: new filings, about a week's worth, newest first.
It uses the same parser as `libfec rss` and Python's `libfec.rss`.

```ts
import { Filing, rss } from "@alex.garcia/libfec";

const feed = await rss.fetch({ forms: ["F3X"], states: "CA" });
for (const item of feed.items) {
  console.log(item.filingId, item.committeeName, item.formType, item.pubDate);
}
const newest = await Filing.fromId(feed.items[0].filingId);
```

- `rss.fetch(options?)` downloads the feed with the runtime's global `fetch()` (no
  dependency), with a 60-second timeout unless you pass a `signal`. A non-2xx response
  throws an `Error`. In a browser, pass a `proxy`, because the FEC sends no CORS headers.
  `proxy` is a URL prefix (`"https://proxy.example/?url="`), a template with `{url}`, or a
  function. See [`@alex.garcia/libfec-wasm`](../fec-wasm/README.md).
- `rss.parseFeed(xml)` parses a feed you fetched yourself (a string or UTF-8 bytes).
- `rss.feedUrl(options?)` gives the URL. `preset` is `"all"` (the default), `"monthly"`,
  `"quarterly"`, `"presidential"`, `"congressional"` or `"pac"`. Any of `committees`,
  `forms`, `states` or `parties` (a string or an iterable of strings) switches to custom
  filters, and the feed then ignores `preset`.

Each item has `filingId`, `committeeId`, `committeeName`, `formType`, `reportType`,
`coverageFrom`/`coverageThrough` (`"YYYY-MM-DD"`), `pubDate` (a `Date`), and the raw
`title`, `link`, `description` and `guid`. `link` is the filing's `.fec` URL, the one
`Filing.fromId(item.filingId)` downloads.

## Known issues

These come from the parser and are the same in the Python package:

- An F99's `[BEGINTEXT]` body is on `filing.cover.text`, but `[BEGINTEXT]` blocks elsewhere
  are skipped.
- A few column names are placeholders ending in `_TODO_DUP`.
- Text that isn't valid UTF-8 is decoded lossily (an invalid byte becomes `U+FFFD`). For
  example, a cp1252 `’` becomes `�`.
- Quotes are mostly literal: one pair of quotes wrapping a whole field comes off (`"O""Brien"`
  reads as `O"Brien`), but the parser doesn't do CSV-style quoting, so a quoted field
  holding a separator or a newline still splits.

## Performance

`benchmarks/node/bench.mjs` on a 91 MB filing (408,160 records), on macOS arm64, measured
before the `Filing` reshape with the passes that `itemizations()` and
`itemizationRecords()` replaced (`itemizations()` and `rows()`):

| | Node 24 | Deno 2.9 | Bun 1.2 |
|---|---|---|---|
| `itemizations()` | 1.31 s | 1.28 s | 1.33 s |
| `itemizationRecords()` | 0.39 s | 0.37 s | 0.41 s |

Typing every record costs about three times the raw path, and most of that is the Rust
parser's typing (~0.6 s), so use `itemizationRecords()` when you don't need typed records.

## Examples

`examples/*.mjs` run under all three runtimes (`make examples`, and in CI). Each takes an
optional filing path and defaults to a committed fixture. `examples/rss.mjs` parses a saved
feed and only fetches the live one when given `--live`. `examples/from-id.mjs` downloads a
filing, so `make examples` skips it; `make examples-network` runs it too.

## Development

```bash
make install            # npm ci
make build              # debug addon (native/) + TypeScript (dist/)
make test               # the node:test suite under node, deno and bun
make test-network       # live requests to fec.gov (Filing.fromId)
make typecheck          # tsc over js/, the compile-only type tests and the examples
make examples           # examples/*.mjs under node, deno and bun (offline)
make examples-network   # the same, plus the examples that fetch from fec.gov
make gen-types          # regenerate js/generated/ from fec-parser (commit the result)
make docs               # TypeDoc -> site/reference/node
make bench              # benchmarks/node/bench.mjs (release build)
make test-slow          # perf guards on benchmarks/1805248.fec
```
