# `libfec`

Work-in-progress parser for [`.fec` files](https://www.fec.gov/help-candidates-and-committees/filing-reports/fecfile-software/). Inspired by:

| Repo                                      | Language      | Release date |
| ----------------------------------------- | ------------- | ------------ |
| https://github.com/cschnaars/FEC-Scraper  | Python+SQLite | ~2011        |
| https://github.com/dwillis/Fech           | Ruby          | ~2012?       |
| https://github.com/PublicI/fec-parse      | Node.js       | ~2015        |
| https://github.com/esonderegger/fecfile   | Python        | ~2018        |
| https://github.com/washingtonpost/FastFEC | C/Python/WASM | ~2021        |

Only FEC filings with version 8.3 and 8.4 are supported, though older versions may work in certain commands.

## Installation

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/asg017/libfec/releases/latest/download/fec-cli-installer.sh | sh
```

Or download the CLI directly from [a recent release](https://github.com/asg017/libfec/releases).

## Usage

### Export itemizations of a FEC filing to a SQLite database

```
libfec export FEC-1813847 -o virginia.db
```

This will download all the itemizations from [this FEC filing]() and export them to a SQLite database at `virginia.db`.
The generated SQLite database will have the following tables:

```
libfec_H1
libfec_H2
libfec_H3
libfec_H4
libfec_SA11AI
libfec_SA11C
libfec_SA12
libfec_SB21B
libfec_SB28A
libfec_SB29
libfec_SB30B
libfec_TEXT
libfec_filings
```

A new table is created for every "form type" for all itemizations. The `libfec_SA*` ones refer to "Schedule A" itemizations, aka "receipts" or contributions.

If you only care about Schedule A itemizations, you can pass in the `--target schedule-a` flag like so:

```
libfec export FEC-1813847 --target schedule-a -o virginia.db
```

Now there will be a single `libfec_schedule_a` that consolidates all Schedule A itemizations into a single table.
This is probably what you want if you're doing stories like "who has donated to this PAC/campaign".

### Export multiple filings in one command

You can provide multiple FEC filing IDs to the `libfec` command line:

```
libfec export FEC-1813847 FEC-1813838 FEC-1813835 --target schedule-a -o project.db
```

Alternatively, you can provide a text file to a list of FEC IDs instead of typing them out one-by-one.
A new ID must appear on it's own line. Blank lines and lines that start with "#" are ignored.

```
# inside input.txt
 FEC-1813847

 FEC-1813838

 FEC-1813835
```

```
libfec export -i input.txt --target schedule-a -o project.db
```

### Export afiling from a file, URL, or ID

You can provide a filing as a file, URL, or ID to `libfec`. If it's a URL or ID, then `libfec` will download it from the FEC website.

All these commands are functionaly equivalent:

```bash
# downloads the FEC-1813847 filings from the fec website
libfec export FEC-1813847 --target schedule-a -o project.db
# uses the already downloaded file called 1813847.fec
libfec export 1813847.fec --target schedule-a -o project.db
# downloads directly from the provided URL
libfec export https://docquery.fec.gov/dcdev/posted/1813847.fec --target schedule-a -o project.db
```
