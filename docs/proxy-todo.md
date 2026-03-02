# Plan: `libfec proxy up` Subcommand

## Context

We need a local HTTP proxy so tools like Claude Code can query the OpenFEC API without being given the real API key. The proxy runs on `localhost:1974`, injects the key, forwards requests, caches responses (reusing existing `SqliteApiCache`), and redacts the key from responses.

## Files to Modify

| File | Action |
|------|--------|
| `crates/fec-cli/Cargo.toml` | Add `tiny_http` dep + `proxy` feature |
| `crates/fec-cli/src/cli.rs` | Add `ProxyArgs`, `ProxySubcommand`, `Commands::Proxy` |
| `crates/fec-cli/src/commands/proxy.rs` | **New** - proxy server implementation |
| `crates/fec-cli/src/commands/mod.rs` | Register proxy module |
| `crates/fec-cli/src/main.rs` | Add dispatch arm |

## Implementation Steps

### 1. Cargo.toml - feature gate + dependency

```toml
[features]
default = ["proxy"]
proxy = ["dep:tiny_http"]

[dependencies]
tiny_http = { version = "0.12", optional = true }
```

`tiny_http` is a minimal sync HTTP server - fits this codebase (no async runtime).

### 2. CLI structs in `cli.rs`

Add `ProxyArgs` with subcommand enum (following `CacheSubcommand` pattern):

- `ProxySubcommand::Up(ProxyUpArgs)`
- `ProxyUpArgs`: `--port` (default 1974), `--api-key` (env `LIBFEC_API_KEY`, fallback `DEMO_KEY`)
- `Commands::Proxy` variant gated with `#[cfg(feature = "proxy")]`

### 3. Proxy server (`commands/proxy.rs`)

Core logic - a single-threaded request loop:

1. Start `tiny_http::Server` on `127.0.0.1:{port}`
2. For each incoming GET request:
   - Build upstream URL: `https://api.open.fec.gov{path}?{query}&api_key={key}`
   - Check `SqliteApiCache` via `cache.get(&url)`
   - On cache miss: `ureq::get()` to upstream, parse `Cache-Control: max-age`, store in cache via `cache.set()`
   - Redact API key from response body string (both raw and URL-encoded forms)
   - Return JSON response with `Content-Type: application/json`
3. Non-GET requests get `405 Method Not Allowed`
4. OPTIONS requests get `204` with CORS headers
5. Errors become `502` JSON error responses

**Key design decision**: We bypass `api_request_cached()` (which requires `pagination`/`results` fields) and interact with `SqliteApiCache` directly via `get()`/`set()`. This lets the proxy handle any arbitrary OpenFEC endpoint. We duplicate the small `parse_cache_control_max_age` logic (8 lines) rather than making it public in `fec-api`.

Cache access: `sourcer.cache.api_cache_mut()` returns `Option<&mut SqliteApiCache>`.

### 4. Wire up in `mod.rs` and `main.rs`

Standard pattern with `#[cfg(feature = "proxy")]` gates.

### 5. `LIBFEC_PROXY_ENABLED=1` support

In `fec-api`'s `Api::new()`: when env var is set, use `http://127.0.0.1:1974` as `base_url` instead of `https://api.open.fec.gov`. The proxy then handles key injection. This lets existing commands transparently route through the proxy.

## Verification

```bash
cargo clippy -p fec-cli
# Start proxy:
libfec proxy up
# Test in another terminal:
curl "http://localhost:1974/v1/filings?committee_id=C00401224&per_page=1"
# Verify: response is valid JSON, no API key in body, second request is cached
```
