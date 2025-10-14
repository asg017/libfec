# The `libfec` Cache

By default, the `libfec` will cache FEC files as they are downloaded. Individual `.fec` files never change, and downloading filings on every export can take a long time. 


The `libfec export` command will automatically save downloaded FEC filings into your cache directory. You can use the `libfec cache --print` command to see where this default `libfec` cache directory is located on your machine:

```bash
$ libfec cache --print
/Users/alex/.cache/libfec/cache
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