## Pre-existing parsers

| Repo                                      | Language      | Release date |
| ----------------------------------------- | ------------- | ------------ |
| https://github.com/cschnaars/FEC-Scraper  | Python+SQLite | ~2011        |
| https://github.com/dwillis/Fech           | Ruby          | ~2012?       |
| https://github.com/PublicI/fec-parse      | Node.js       | ~2015        |
| https://github.com/esonderegger/fecfile   | Python        | ~2018        |
| https://github.com/washingtonpost/FastFEC | C/Python/WASM | ~2021        |

https://github.com/dwillis/fech-sources

## files themselves

https://docquery.fec.gov/dcdev/posted/13360.fec

https://docquery.fec.gov/dcdev/posted/1272203.fec

```
fec-to-sqlite filings filings.db 13360
```

## Format spelunking

Looking at `13360.fec`, at this line:

```
SA11A1,C00101766,IND,Kellner^Lawrence,10915 Pifer Way,,Houston,TX,77024,,,"Continental Airlines, Inc.",Exec. V.P. & CFO,5000.00,20000510,5000.00,,,,,,,,,,,,,,,,,A,SA11A1.7430
```

I ran `fastfec 13360`, and the above line landed at `output/13360/SA11A1.csv`.

The `mappings.json` path looks like `"^sa[^3]"` -> `"^3"`, based on the CSV headers. How did that happen?

- `^sa[^3]` matches the first line `SA11A1`
- The second line is a ""

`mappings.json` level 1: the "form", the first field in the CSV row. Level 2 is the "version"
'

## INfo

https://github.com/ryanpitts/journalists-guide-datasets/blob/34f467d0ec5a79ea02c8ef8acd9361aeebadc005/datasets/federal_election_commission.md

https://docquery.fec.gov/dcdev/posted/13360.fec
https://docquery.fec.gov/dcdev/posted/1795717.fec

https://cg-519a459a-0ea3-42c2-b7bc-fa1143481f74.s3-us-gov-west-1.amazonaws.com/bulk-downloads/index.html

https://cg-519a459a-0ea3-42c2-b7bc-fa1143481f74.s3-us-gov-west-1.amazonaws.com/bulk-downloads/index.html?prefix=bulk-downloads/electronic/



BEGINTEXT utf8 error sample:

```
rm june.db; cargo run export june/1787785.fec -o june.db
```



result:

```
➜ rm june.db; ./target/release/fec-cli export june/*.fec -o june.db
[00:00:04] █░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░     261/5805    june/1787062.fec                        Warning too long 209 vs 207!
[00:08:35] ███████████████████████████████░░░░░░░░░    4606/5805    june/1791562.fec                        Warning too long 62 vs 46!
[00:11:41] ███████████████████████████████░░░░░░░░░    4606/5805    june/1791562.fec                        Warning too long 62 vs 46!
[00:51:25] ████████████████████████████████████████    5805/5805    june/1792770.fec

june.db: 2.6GB
4805 filings
96 tables
SA18 - 987k rows
SA11AO - 976k rows
```
