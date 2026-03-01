# The `libfec` Cache

By default, the `libfec` will cache FEC files as they are downloaded. Individual `.fec` files never change, and downloading filings on every export can take a long time. 


The `libfec export` command will automatically save downloaded FEC filings into your cache directory. You can use the `libfec cache info` command to see where this default `libfec` cache directory is located on your machine:

```bash
$ libfec cache info
```
```
Cache Information

  /Users/alex/.cache/libfec/cache

  FEC filings cached:    34,295 .fec files (15.41 GiB)
  Daily zip meta files:  335
  Bulk data database:    140.45 MiB  /Users/alex/.cache/libfec/cache/.bulk-data.db
  API cache database:    73.56 MiB  /Users/alex/.cache/libfec/cache/.api-cache.db

  Bulk Data
    Candidates (2026)
      2026 — modified 9 hours ago, checked 4 minutes ago
    Committees (2026)
      2026 — modified 9 hours ago, checked 4 minutes ago
    Candidate-Committee Linkages (2022, 2024, 2026)
      2026 — modified 1 day ago, checked 12 hours ago
    Operating Expenditures (2026)
      2026 — modified 24 days ago, checked 22 days ago
```

## Changing the cache directory

You can configure the cache directory in two ways — first with the `LIBFEC_CACHE_DIRECTORY` environment variable:

```bash
LIBFEC_CACHE_DIRECTORY=$PWD/cache libfec export \
  --election 2024 CA40 \
  -o ca40.db
```

This will create a new `cache/` directory and cache 30 matching filings into this folder. 

Alternatively, you can use the `--cache-directory` flag:

```bash
libfec export \
  --cache-directory=cache \
  --election 2024 CA40 \
  -o ca40.db
```