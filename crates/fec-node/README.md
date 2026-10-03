# @asg017/libfec

Parse FEC electronic filings (`.fec`) in Node, Deno and Bun, with libfec's Rust parser
via [napi-rs](https://napi.rs). **Pre-alpha:** the API is being built
(`plans/nodejs/`); only `version` and `readHeader` exist so far.

## Development

```bash
make install   # npm ci
make build     # debug addon (native/) + TypeScript (dist/)
make test      # the same node:test suite under node, deno and bun
```

All three runtimes report Node-API 10 (`process.versions.napi`: Node 24.12, Deno 2.9.5,
Bun 1.2.19); the addon needs 8. Deno needs `--allow-read --allow-ffi --allow-env` (the
generated loader reads `NAPI_RS_*` env vars).
