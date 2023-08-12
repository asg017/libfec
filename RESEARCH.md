## Pre-existing parsers

| Repo                                      | Language      | Release date |
| ----------------------------------------- | ------------- | ------------ |
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