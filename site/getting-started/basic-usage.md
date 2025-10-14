# Basic `libfec` Usage

Once you have [installed the `libfec` CLI](./installation.md), you can begin to export FEC filings into various file formats like CSVs, JSON, SQLite, or Excel.

## Basic exports 

Let's export all data from Nikki Haley's 2023 Q1 presidential campaign's filing, [`FEC-1721616`](https://docquery.fec.gov/cgi-bin/forms/C00833392/1721616):

```bash
libfec export \
  FEC-1721616 \
  -o haley-2023-Q1.db
```
```output
Exporting filings to SQLite database at "haley-2023-Q1.db"
Finished exporting 1 filings into haley-2023-Q1.db, in 0 seconds
```

Let's break down this command:

- `libfec export`: We're using the `libfec` program to call the `export` command, which exports FEC filings into different file formats.
- `FEC-1721616`: This is the positional argument for the `export` subcommand, the ID of the FEC filing we care about (Nikki Haley's 2023 Q1 financial report). 
- `-o haley-2023-Q1.db`: The "output" flag, meaning we want to output data into the `haley-2023-Q1.db` file. The `.db` file extension designates this as a SQLite export.

So the result is a single `haley-2023-Q1.db` SQLite database file, with various generated tables for each itemization:

```bash
› sqlite3 haley-2023-Q1.db
sqlite> .tables
libfec_F3P         libfec_filings     libfec_schedule_a  libfec_schedule_b
```

You can then use SQL to query the exact into you're looking for, using your programming language's SQLite client library. See [SQL Reference](../reference/sql.md) for more info about these tables.

## Exporting to CSV

There are two types of CSV exports that `libfec` supports: **CSV directory exports** and **target specific" CSV exports**.

CSV directory exports are made with the `--output-directory` and `--format csv` flags:

```bash
$ libfec export \
  FEC-1721616 \
  --format csv \
  --output-directory haley-2023-Q1
$ tree --du -h haley-2023-Q1/
```

```output
[5.4M]  haley-2023-Q1/
├── [5.3K]  cover_F3P.csv
├── [5.4M]  schedule_a.csv
└── [ 43K]  schedule_b.csv
```

Itemizations from the `FEC-1721616` filing will be exported by itemization type into various CSVs inside of `haley-2023-Q1`. In this example, `cover_F3P.csv` will have the `F3P` summary page information from the filing. The `schedule_a.csv` file contains receipt information including individual contributors and PAC contributions. The `schedule_b.csv` file contains disbursement information, like operating expenses, transfers to other PACs, refunds, and more.

If instead you only care about receipts, you can instead perform a target-sepcfic CSV export like so:

```bash
$ libfec export \
  FEC-1721616 \
  --target receipts \
  -o haley-2023-Q1-receipts.csv
```

The `--target receipts` flag  will export only the Schedule A itemizations records from `FEC-1721616` into a single CSV file at `haley-2023-Q1-receipts.csv`.

Alternatively use `--target disbursements` for Schedule B itemizations:

```bash
$ libfec export \
  FEC-1721616 \
  --target disbursements \
  -o haley-2023-Q1-disbursements.csv
```

## Exporting a candidate or committee

Copy+pasting FEC filing IDs can get old after a while! Instead of exporting a single FEC filing, you can also provide a candidate ID or committee ID, and `libfec` will find all relevent filings.

For example, let's export all filings from the 2024 cycle of Congresswoman Young Kim's principal campaign committee, [Young Kim for Congress (`C00665638`)](https://www.fec.gov/data/committee/C00665638/):

```bash
libfec export \
  --cycle 2024 \
  C00665638 \
  -o kim.db
```

Now `kim.db` contains data from 12 filings submitted by Kim's committee, relevant to the 2024 election cycle. `libfec` will automatically use the [OpenFEC API](https://api.open.fec.gov/developers/) to find relevant filings. 


## Exporting all candidates in an election

`libfec` can also resolve filings from a specific presidential, senate, or house race. Say we wanted to get all filings from all canddiate committees who ran in the [California 40th district](https://en.wikipedia.org/wiki/California%27s_40th_congressional_district) 2024 race, which is the current district Kim represents. We can get that like so:

```bash
libfec export \
  --election 2024 \
  CA40 \
  -o ca40-2024.db
```

The `--election` flag will find all candidates who ran in the 2024 CA40 election, and export all of their filings into `ca40-2024.db`. 

You can do the same for senate elections:

```bash
# Iowa senate candidates for 2026
libfec export \
  --election 2026 \
  senate-IA \
  -o ia-2026.db
```

Or presidents:

```bash
libfec export \
  --election 2024 \
  president \
  -o prez-2024.db
```

## Form Type Filtering

Candidates and committees file several different types of forms and reports to the FEC. You can filter for exact forms that you care about with the `--form-type` flag.

For example, candidates file F2 forms to declare their candidacy. To export all F2 forms filed by people who ran for the California senate seat in 2024, you can run:

```bash
libfec export \
  --cycle 2024 \
  --form-type F2 \
  --state CA \
  --office S \
  -o ca-senate-2024.db
```